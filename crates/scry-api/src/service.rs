use crate::error::ServiceError;
use chrono::Utc;
use scry_core::error::StorageError;
use scry_core::ids::{
    AutomationId, Handle, MIN_HANDLE_LENGTH, NoteId, ProjectId, SectionId, TaskId,
    distinguishing_prefixes, shortest_handle, shortest_handles, template_section_id,
};
use scry_core::models::{
    Automation, AutomationAction, AutomationTrigger, Change, ChangeKind, Color, Entity, EntityType,
    ExternalIdentity, ExternalMetadata, FieldValues, MAX_AUTOMATION_ACTIONS, Note, Priority,
    Project, ProjectSnapshot, ProjectTemplate, Section, Tags, Task, TaskSortingMode, TaskState,
};
use scry_core::position::{Position, key_between, n_keys_between};
use scry_core::snapshot::{
    self, ImportMode, ImportReport, SNAPSHOT_VERSION, Snapshot, SnapshotFile,
};
use scry_core::store::ArcStore;
use scry_core::sync::ack::{
    AcknowledgmentStatus, ApplyOutcome, OperationAcknowledgment, RejectionReason,
};
use scry_core::sync::operations::{
    ColorChange as OperationColorChange, OPERATION_VERSION, Operation, OperationKind,
};
use serde_json::{Map, Value, json};
use sqlx::{PgConnection, Postgres, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// How an operation selects a task's section: leave it alone, clear it, use an
/// existing section, or create one by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SectionInput {
    #[default]
    Unchanged,
    NoSection,
    Existing(SectionId),
    New(String),
}

fn position_error(error: scry_core::position::PositionError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(format!(
        "failed to generate position: {error}"
    )))
}

/// Build a wire operation for a REST write. REST writes carry no client
/// timestamp (#46); the server's time is recorded on the change row.
fn new_operation(operation_id: Uuid, kind: OperationKind) -> Operation {
    Operation {
        operation_id,
        operation_version: OPERATION_VERSION,
        timestamp: None,
        kind,
    }
}

/// Turn a pipeline rejection into the error a REST write returns. Tombstoned
/// entities are reported as not found because they no longer exist.
fn rejection_error(operation_id: Uuid, reason: RejectionReason) -> ServiceError {
    let message = format!("operation {operation_id} rejected: {}", reason.as_str());
    ServiceError::Storage(match reason {
        RejectionReason::NotFound | RejectionReason::Tombstoned => StorageError::NotFound(message),
        RejectionReason::Conflict => StorageError::Conflict(message),
        RejectionReason::UnsupportedVersion => StorageError::Invalid(message),
    })
}

/// How a batch of operations treats a rejected operation.
#[derive(Debug, Clone, Copy)]
pub enum ApplyMode {
    /// Roll the whole batch back on the first rejection (REST writes).
    Atomic,
    /// Acknowledge the rejection and apply the rest of the batch (#104's `/sync`).
    PerOperation,
}

/// Whether applying an operation should evaluate the project's automations
/// after the operation takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationEvaluation {
    /// Normal user and server writes: evaluate automations after applying.
    Evaluate,
    /// Import and automation-generated actions: do not evaluate automations.
    Skip,
}

/// The acks, pulled changes, and next cursor produced by one sync round.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub acks: Vec<OperationAcknowledgment>,
    pub changes: Vec<Change>,
    pub next_seq: i64,
}

/// Input for creating a task. `SectionInput::Unchanged` means no section.
#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
    pub external_identity: Option<ExternalIdentity>,
    pub external_metadata: Option<ExternalMetadata>,
}

/// Partial task input. `None` leaves a field unchanged, including
/// `SectionInput::Unchanged` for the section; `Some("")` on `description`
/// clears it; `Some(Tags)` on `tags` sets them (an empty `Tags` clears them).
#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
}

/// A task after a create, duplicate, or move.
#[derive(Debug, Clone)]
pub struct TaskChangeResult {
    pub task: Task,
}

/// Partial project input. `None` leaves a field unchanged; `Some("")` on
/// `description` clears it.
#[derive(Debug, Clone, Default)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
    pub task_sorting_mode: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
}

/// How an update changes a section's color.
#[derive(Debug, Clone, Copy)]
pub enum ColorChange {
    /// Remove the section's color.
    Clear,
    /// Set the section's color.
    Set(Color),
}

/// Partial section input. `None` leaves the name unchanged.
#[derive(Debug, Clone)]
pub struct UpdateSection {
    pub name: Option<String>,
    pub color: ColorChange,
}

/// Where to place a section relative to its siblings.
#[derive(Debug, Clone, Copy)]
pub enum SectionPlacement {
    Top,
    Bottom,
    Before(SectionId),
    After(SectionId),
}

/// Where to place a task relative to its siblings in the destination group.
#[derive(Debug, Clone, Copy)]
pub enum TaskPlacement {
    Top,
    Bottom,
    Before(TaskId),
    After(TaskId),
}

/// How a new project's sorting mode, priority flag, and sections are chosen.
#[derive(Debug, Clone, Copy)]
pub enum ProjectSetup {
    /// Adopt the template's sorting mode, priority flag, and sections.
    Template(ProjectTemplate),
    /// Use these settings and seed no sections.
    Explicit {
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    },
}

/// Frontend-agnostic, high-level operations over one project. Resolves names to
/// ids, picks defaults, computes positions, and enforces invariants before
/// delegating the raw writes to `Store`.
#[derive(Clone)]
pub struct ProjectService {
    store: ArcStore,
}

impl ProjectService {
    pub fn new(store: ArcStore) -> Self {
        Self { store }
    }

    pub async fn create_task_in_project(
        &self,
        project_id: ProjectId,
        input: NewTask,
    ) -> Result<TaskChangeResult, ServiceError> {
        let project = self.require_project(project_id).await?;
        let (section, section_op) = self.resolve_section(&project, input.section).await?;
        let section_id = section.as_ref().map(|section| section.id);
        let position = self.append_position(project.id, section_id).await?;

        let task = Task {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: input.title,
            description: input
                .description
                .filter(|description| !description.is_empty()),
            priority: input.priority.unwrap_or_default(),
            section_id,
            position: position.clone(),
            state: TaskState::Open,
            tags: input.tags.unwrap_or_default(),
            created_at: Utc::now(),
            external_identity: input.external_identity,
            external_metadata: input.external_metadata,
        };

        let mut ops = Vec::new();
        if let Some(section_op) = section_op {
            ops.push(section_op);
        }
        ops.push(new_operation(
            Uuid::new_v4(),
            OperationKind::CreateTask {
                id: task.id,
                project_id: task.project_id,
                title: task.title.clone(),
                description: task.description.clone(),
                priority: task.priority,
                section_id: task.section_id,
                position,
                state: task.state,
                tags: task.tags.clone(),
                created_at: task.created_at,
                external_identity: task.external_identity.clone(),
                external_metadata: task.external_metadata.clone().map(Box::new),
            },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn duplicate_task(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<TaskChangeResult, ServiceError> {
        let source = self.require_task_in_project(project, id).await?;
        let tasks = self.tasks_in_section(project.id, source.section_id).await?;
        let next_position = tasks
            .iter()
            .position(|task| task.id == source.id)
            .and_then(|index| tasks.get(index + 1))
            .map(|task| &task.position);
        let position =
            key_between(Some(&source.position), next_position).map_err(position_error)?;

        let task = Task {
            id: Uuid::new_v4(),
            project_id: source.project_id,
            title: source.title,
            description: source.description,
            priority: source.priority,
            section_id: source.section_id,
            position: position.clone(),
            state: source.state,
            tags: source.tags,
            created_at: Utc::now(),
            external_identity: source.external_identity.clone(),
            external_metadata: source.external_metadata.clone(),
        };

        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateTask {
                id: task.id,
                project_id: task.project_id,
                title: task.title.clone(),
                description: task.description.clone(),
                priority: task.priority,
                section_id: task.section_id,
                position,
                state: task.state,
                tags: task.tags.clone(),
                created_at: task.created_at,
                external_identity: task.external_identity.clone(),
                external_metadata: source.external_metadata.clone().map(Box::new),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn list_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self
            .store
            .get_all_tasks_by_project_id(project_id, None)
            .await?)
    }

    pub async fn list_tasks_by_section_id(
        &self,
        section_id: SectionId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self
            .store
            .get_all_tasks_by_section_id(section_id, None)
            .await?)
    }

    pub async fn update_task_by_id(
        &self,
        task_id: TaskId,
        patch: TaskPatch,
    ) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        let mut ops = Vec::new();
        let mut moved = false;

        if patch.section != SectionInput::Unchanged {
            let (section, section_op) = self.resolve_section(&project, patch.section).await?;
            if let Some(section_op) = section_op {
                ops.push(section_op);
            }
            let section_id = section.as_ref().map(|section| section.id);
            if section_id != task.section_id {
                task.position = self.append_position(project.id, section_id).await?;
                task.section_id = section_id;
                moved = true;
            }
        }

        if moved {
            ops.push(new_operation(
                Uuid::new_v4(),
                OperationKind::MoveTask {
                    id: task.id,
                    section_id: task.section_id,
                    position: task.position.clone(),
                },
            ));
        }

        if patch.title.is_some()
            || patch.description.is_some()
            || patch.priority.is_some()
            || patch.tags.is_some()
        {
            let description = patch
                .description
                .map(|value| Some(value).filter(|value| !value.is_empty()));
            let kind = OperationKind::UpdateTask {
                id: task.id,
                title: patch.title,
                description,
                priority: patch.priority,
                tags: patch.tags,
                state: None,
            };
            // apply exactly the op's fields to the returned model
            task_field_updates(&mut task, &kind);
            ops.push(new_operation(Uuid::new_v4(), kind));
        }

        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(task)
    }

    pub async fn move_task_by_id(
        &self,
        task_id: TaskId,
        section: SectionInput,
        placement: Option<TaskPlacement>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        let unchanged = section == SectionInput::Unchanged;
        let (section, section_op) = if unchanged {
            match task.section_id {
                Some(section_id) => (
                    Some(
                        self.require_section_in_project(&project, section_id)
                            .await?,
                    ),
                    None,
                ),
                None => (None, None),
            }
        } else {
            self.resolve_section(&project, section).await?
        };

        if unchanged && placement.is_none() {
            return Ok(TaskChangeResult { task });
        }

        let section_id = section.as_ref().map(|section| section.id);

        let position = match placement {
            Some(placement) => {
                if let TaskPlacement::Before(anchor_id) | TaskPlacement::After(anchor_id) =
                    placement
                {
                    if anchor_id == task.id {
                        return Err(ServiceError::TaskMoveAnchorItself { task: task.id });
                    }
                    let anchor = self.require_task_in_project(&project, anchor_id).await?;
                    if anchor.section_id != section_id {
                        let section_name = section
                            .as_ref()
                            .map(|section| section.name.clone())
                            .unwrap_or_else(|| "No section".to_string());
                        return Err(ServiceError::TaskAnchorNotInSection {
                            task: anchor_id,
                            section: section_name,
                        });
                    }
                }
                let tasks = self.tasks_in_section(project.id, section_id).await?;
                match task_placement_position(&tasks, &task, placement)? {
                    Some(position) => position,
                    None => return Ok(TaskChangeResult { task }),
                }
            }
            None => self.append_position(project.id, section_id).await?,
        };
        task.position = position;
        task.section_id = section_id;

        let mut ops = Vec::new();
        if let Some(section_op) = section_op {
            ops.push(section_op);
        }
        ops.push(new_operation(
            Uuid::new_v4(),
            OperationKind::MoveTask {
                id: task.id,
                section_id: task.section_id,
                position: task.position.clone(),
            },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn close_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        if task.state == TaskState::Closed {
            return Ok(task);
        }
        task.state = TaskState::Closed;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateTask {
                id: task.id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: Some(task.state),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(task)
    }

    pub async fn open_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        if task.state == TaskState::Open {
            return Ok(task);
        }
        task.state = TaskState::Open;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateTask {
                id: task.id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: Some(task.state),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(task)
    }

    pub async fn delete_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteTask { id: task_id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(task)
    }

    pub async fn add_note(&self, task_id: TaskId, contents: String) -> Result<Note, ServiceError> {
        self.require_task(task_id).await?;
        let note = Note {
            id: Uuid::new_v4(),
            task_id,
            contents,
            created_at: Utc::now(),
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateNote {
                id: note.id,
                task_id: note.task_id,
                contents: note.contents.clone(),
                created_at: note.created_at,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(note)
    }

    pub async fn list_notes_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, ServiceError> {
        Ok(self
            .store
            .get_all_notes_by_project_id(project_id, None)
            .await?)
    }

    pub async fn list_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, ServiceError> {
        Ok(self.store.get_all_notes_by_task_id(task_id, None).await?)
    }

    pub async fn get_note(&self, id: NoteId) -> Result<Option<Note>, ServiceError> {
        Ok(self.store.get_note_by_id(id, None).await?)
    }

    pub async fn update_note(&self, note: Note) -> Result<Note, ServiceError> {
        let existing = self.require_note(note.id).await?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateNote {
                id: note.id,
                contents: note.contents.clone(),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(Note {
            contents: note.contents,
            ..existing
        })
    }

    pub async fn delete_note(&self, id: NoteId) -> Result<(), ServiceError> {
        self.require_note(id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteNote { id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(())
    }

    // display handles are unique per entity type across the whole database,
    // including closed tasks and every project, so they need the complete id
    // set; display always starts at the 4-character floor
    pub async fn list_task_handles(&self) -> Result<HashMap<TaskId, String>, ServiceError> {
        let ids = self.store.get_all_task_ids(None).await?;
        Ok(shortest_handles(ids, MIN_HANDLE_LENGTH))
    }

    // display starts at the 4-character floor
    pub async fn get_task_handle(&self, id: TaskId) -> Result<String, ServiceError> {
        let ids = self.store.get_all_task_ids(None).await?;
        Ok(shortest_handle(id, ids, MIN_HANDLE_LENGTH))
    }

    pub async fn get_task_by_handle(&self, handle: &Handle) -> Result<Task, ServiceError> {
        let matches = self.store.find_task_ids_by_handle(handle, 2, None).await?;
        match matches.len() {
            0 => Err(ServiceError::TaskHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_task(matches[0]).await,
            _ => {
                let all = self
                    .store
                    .find_task_ids_by_handle(handle, i64::MAX, None)
                    .await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousTaskHandle {
                    handle: handle.to_string(),
                    candidates,
                })
            }
        }
    }

    // the current API has no note-prefix entry point; this exists so note handles
    // resolve like task handles wherever note ids are later accepted
    #[allow(dead_code)]
    pub async fn get_note_by_handle(&self, handle: &Handle) -> Result<Note, ServiceError> {
        let matches = self.store.find_note_ids_by_handle(handle, 2, None).await?;
        match matches.len() {
            0 => Err(ServiceError::NoteHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_note(matches[0]).await,
            _ => {
                let all = self
                    .store
                    .find_note_ids_by_handle(handle, i64::MAX, None)
                    .await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousNoteHandle {
                    handle: handle.to_string(),
                    candidates,
                })
            }
        }
    }

    async fn build_section(
        &self,
        project: &Project,
        name: String,
        color: Option<Color>,
    ) -> Result<(Section, Operation), ServiceError> {
        let name = self.validated_section_name(project, None, name).await?;
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        let last = sections
            .iter()
            .map(|section| &section.position)
            .max()
            .cloned();
        let position = key_between(last.as_ref(), None).map_err(position_error)?;
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let section = Section {
            id,
            project_id: project.id,
            name: name.clone(),
            position: position.clone(),
            color,
            created_at,
        };
        let operation = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateSection {
                id,
                project_id: project.id,
                name,
                position,
                color,
                created_at,
            },
        );
        Ok((section, operation))
    }

    pub async fn create_section_in_project(
        &self,
        project_id: ProjectId,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let project = self.require_project(project_id).await?;
        let (section, operation) = self.build_section(&project, name, color).await?;
        self.apply_operations(&[operation], ApplyMode::Atomic)
            .await?;
        Ok(section)
    }

    /// Look up a section by its exact name within a project.
    pub async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Section, ServiceError> {
        match self
            .store
            .get_section_by_name(project_id, name, None)
            .await?
        {
            Some(section) => Ok(section),
            None => {
                let project = self.require_project(project_id).await?;
                Err(ServiceError::SectionNotFound {
                    section: name.to_string(),
                    project: project.name,
                })
            }
        }
    }

    pub async fn get_section(&self, id: SectionId) -> Result<Option<Section>, ServiceError> {
        Ok(self.store.get_section_by_id(id, None).await?)
    }

    pub async fn list_sections_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Section>, ServiceError> {
        Ok(self
            .store
            .get_all_sections_by_project_id(project_id, None)
            .await?)
    }

    /// Update a section's name and color, leaving untouched fields alone.
    pub async fn update_section(
        &self,
        section_id: SectionId,
        update: UpdateSection,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let name = match update.name {
            Some(name) => {
                let project = self.require_project(section.project_id).await?;
                self.validated_section_name(&project, Some(section.id), name)
                    .await?
            }
            None => section.name.clone(),
        };
        let color = match update.color {
            ColorChange::Clear => None,
            ColorChange::Set(color) => Some(color),
        };
        let operation_color = match update.color {
            ColorChange::Clear => OperationColorChange::Clear,
            ColorChange::Set(color) => OperationColorChange::Set(color),
        };
        let updated = Section {
            name: name.clone(),
            color,
            ..section
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateSection {
                id: updated.id,
                name: Some(name),
                color: Some(operation_color),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(updated)
    }

    /// Move a section to the top, bottom, or next to a sibling.
    pub async fn move_section(
        &self,
        section_id: SectionId,
        placement: SectionPlacement,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let project = self.require_project(section.project_id).await?;
        let anchor_id = match placement {
            SectionPlacement::Before(anchor_id) | SectionPlacement::After(anchor_id) => {
                Some(anchor_id)
            }
            _ => None,
        };
        if let Some(anchor_id) = anchor_id {
            self.require_section_in_project(&project, anchor_id).await?;
            if anchor_id == section.id {
                return Err(ServiceError::SectionMoveAnchorItself {
                    section: section.name.clone(),
                });
            }
        }
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        let Some(new_position) = section_placement_position(&sections, &section, placement)? else {
            return Ok(section);
        };
        let updated = Section {
            position: new_position.clone(),
            ..section
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::MoveSection {
                id: updated.id,
                position: new_position,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(updated)
    }

    pub async fn delete_section_by_id(
        &self,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let automations = self
            .store
            .get_all_automations_by_project_id(section.project_id, None)
            .await?;
        if automations
            .iter()
            .any(|automation| automation.references_section(section.id))
        {
            return Err(ServiceError::SectionInUseByAutomation {
                section: section.name.clone(),
            });
        }
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteSection { id: section_id },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(section)
    }

    pub async fn merge_sections_by_id(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.require_section(source_id).await?;
        let project = self.require_project(source.project_id).await?;
        let target = self.require_section_in_project(&project, target_id).await?;
        if source.id == target.id {
            return Err(ServiceError::SectionMergeIntoItself {
                section: source.name.clone(),
            });
        }

        let automations = self
            .store
            .get_all_automations_by_project_id(project.id, None)
            .await?;
        if let Some(automation) = automations.iter().find(|automation| {
            automation.references_section(source.id) || automation.references_section(target.id)
        }) {
            let section = if automation.references_section(source.id) {
                source.name.clone()
            } else {
                target.name.clone()
            };
            return Err(ServiceError::SectionInUseByAutomation { section });
        }

        let source_tasks = self.tasks_in_section(project.id, Some(source.id)).await?;
        let last_target = self.last_task_position(project.id, Some(target.id)).await?;
        let positions = n_keys_between(last_target.as_ref(), None, source_tasks.len())
            .map_err(position_error)?;
        let mut ops = Vec::with_capacity(source_tasks.len() + 1);
        for (task, position) in source_tasks.into_iter().zip(positions) {
            ops.push(new_operation(
                Uuid::new_v4(),
                OperationKind::MoveTask {
                    id: task.id,
                    section_id: Some(target.id),
                    position,
                },
            ));
        }
        ops.push(new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteSection { id: source.id },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok((source, target))
    }

    /// Create an automation in a project.
    pub async fn create_automation_in_project(
        &self,
        project_id: ProjectId,
        trigger: AutomationTrigger,
        actions: Vec<AutomationAction>,
    ) -> Result<Automation, ServiceError> {
        let project = self.require_project(project_id).await?;
        let automation = Automation {
            id: Uuid::new_v4(),
            project_id: project.id,
            trigger,
            actions: actions.clone(),
            created_at: Utc::now(),
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateAutomation {
                id: automation.id,
                project_id: automation.project_id,
                trigger: automation.trigger.clone(),
                actions,
                created_at: automation.created_at,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(automation)
    }

    /// Replace an automation's action list. The trigger is immutable.
    pub async fn update_automation(
        &self,
        id: AutomationId,
        actions: Vec<AutomationAction>,
    ) -> Result<Automation, ServiceError> {
        let existing = self
            .store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateAutomation {
                id,
                actions: actions.clone(),
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(Automation {
            actions,
            ..existing
        })
    }

    /// Delete an automation, returning the deleted rule.
    pub async fn delete_automation(&self, id: AutomationId) -> Result<Automation, ServiceError> {
        let existing = self
            .store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteAutomation { id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(existing)
    }

    pub async fn get_automation(
        &self,
        id: AutomationId,
    ) -> Result<Option<Automation>, ServiceError> {
        Ok(self.store.get_automation_by_id(id, None).await?)
    }

    pub async fn list_automations(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Automation>, ServiceError> {
        Ok(self
            .store
            .get_all_automations_by_project_id(project_id, None)
            .await?)
    }

    /// Check an automation against the project's sections and sibling rules.
    ///
    /// Returns `Ok(None)` when valid, or the reason the operation should be
    /// rejected. `exclude` is the rule being updated, which is not a duplicate
    /// of itself.
    async fn validate_automation(
        &self,
        project: &Project,
        trigger: &AutomationTrigger,
        actions: &[AutomationAction],
        exclude: Option<AutomationId>,
        conn: &mut PgConnection,
    ) -> Result<Option<RejectionReason>, ServiceError> {
        if actions.is_empty() || actions.len() > MAX_AUTOMATION_ACTIONS {
            return Ok(Some(RejectionReason::Conflict));
        }

        let section_ids = trigger
            .section_id()
            .into_iter()
            .chain(actions.iter().filter_map(AutomationAction::section_id));
        for section_id in section_ids {
            let Some(section) = self
                .store
                .get_section_by_id(section_id, Some(&mut *conn))
                .await?
            else {
                return Ok(Some(RejectionReason::NotFound));
            };
            if section.project_id != project.id {
                return Ok(Some(RejectionReason::NotFound));
            }
            if self
                .store
                .is_tombstoned(EntityType::Section, section_id, Some(&mut *conn))
                .await?
            {
                return Ok(Some(RejectionReason::Tombstoned));
            }
        }

        let rules = self
            .store
            .get_all_automations_by_project_id(project.id, Some(&mut *conn))
            .await?;
        if rules
            .iter()
            .any(|rule| Some(rule.id) != exclude && rule.trigger == *trigger)
        {
            return Ok(Some(RejectionReason::Conflict));
        }

        Ok(None)
    }

    /// Create a project, optionally seeding sections from a template.
    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
        setup: ProjectSetup,
    ) -> Result<Project, ServiceError> {
        let (task_sorting_mode, show_priority, template_sections) = match setup {
            ProjectSetup::Template(template) => (
                template.task_sorting_mode,
                template.show_priority,
                Some(template.sections),
            ),
            ProjectSetup::Explicit {
                task_sorting_mode,
                show_priority,
            } => (task_sorting_mode, show_priority, None),
        };

        let project = Project {
            id: Uuid::new_v4(),
            name,
            description: description.filter(|description| !description.is_empty()),
            task_sorting_mode,
            show_priority,
            created_at: Utc::now(),
        };

        if self.get_project_by_name(&project.name).await?.is_some() {
            return Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                project.name
            ))
            .into());
        }

        let mut ops = vec![new_operation(
            Uuid::new_v4(),
            OperationKind::CreateProject {
                id: project.id,
                name: project.name.clone(),
                description: project.description.clone(),
                task_sorting_mode: project.task_sorting_mode,
                show_priority: project.show_priority,
                created_at: project.created_at,
            },
        )];

        if let Some(sections) = template_sections {
            for section in sections {
                let position = Position::parse(section.position).map_err(position_error)?;
                let id = template_section_id(project.id, section.name);
                ops.push(new_operation(
                    Uuid::new_v4(),
                    OperationKind::CreateSection {
                        id,
                        project_id: project.id,
                        name: section.name.to_string(),
                        position,
                        color: section.color,
                        created_at: Utc::now(),
                    },
                ));
            }
        }

        self.apply_operations(&ops, ApplyMode::Atomic).await?;
        Ok(project)
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.store.get_all_projects(None).await?)
    }

    pub async fn get_project(&self, id: ProjectId) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_id(id, None).await?)
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_name(name, None).await?)
    }

    pub async fn project_snapshot(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectSnapshot, ServiceError> {
        Ok(self.store.get_project_snapshot(project_id, None).await?)
    }

    pub async fn update_project(
        &self,
        project: &Project,
        update: UpdateProject,
    ) -> Result<Project, ServiceError> {
        let description = match update.description.clone() {
            Some(value) => Some(value).filter(|value| !value.is_empty()),
            None => project.description.clone(),
        };
        let updated = Project {
            name: update.name.clone().unwrap_or_else(|| project.name.clone()),
            description,
            task_sorting_mode: update
                .task_sorting_mode
                .unwrap_or(project.task_sorting_mode),
            show_priority: update.show_priority.unwrap_or(project.show_priority),
            ..project.clone()
        };

        let changed = update.name.is_some()
            || update.description.is_some()
            || update.task_sorting_mode.is_some()
            || update.show_priority.is_some();
        if changed {
            let description = update
                .description
                .map(|value| Some(value).filter(|value| !value.is_empty()));
            let op = new_operation(
                Uuid::new_v4(),
                OperationKind::UpdateProject {
                    id: project.id,
                    name: update.name,
                    description,
                    task_sorting_mode: update.task_sorting_mode,
                    show_priority: update.show_priority,
                },
            );
            self.apply_operations(&[op], ApplyMode::Atomic).await?;
        }
        Ok(updated)
    }

    /// Delete a project by id, returning the deleted project.
    pub async fn delete_project_by_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Project, ServiceError> {
        let project = self.require_project(project_id).await?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteProject { id: project.id },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(project)
    }

    /// Read one consistent server-wide snapshot plus its cursor.
    pub async fn snapshot(&self, project: Option<ProjectId>) -> Result<Snapshot, ServiceError> {
        Ok(snapshot::build(self.store.as_ref(), project).await?)
    }

    /// Wrap a snapshot in the exportable file envelope. `include_seq` adds the
    /// cursor to the file; ordinary exports leave it out.
    pub async fn snapshot_file(
        &self,
        project: Option<ProjectId>,
        include_seq: bool,
    ) -> Result<SnapshotFile, ServiceError> {
        let snapshot = self.snapshot(project).await?;
        Ok(SnapshotFile {
            version: SNAPSHOT_VERSION,
            app_version: String::new(),
            projects: snapshot.projects,
            last_seen_seq: include_seq.then_some(snapshot.last_seen_seq),
        })
    }

    /// Apply a snapshot file in one atomic batch.
    pub async fn import(
        &self,
        file: &SnapshotFile,
        mode: ImportMode,
    ) -> Result<ImportReport, ServiceError> {
        let mut tx = self.store.begin_apply().await?;
        let state = self.load_import_state(&mut tx, file).await?;
        let plan = snapshot::plan_import(file, &state, mode)?;
        self.apply_batch(
            &mut tx,
            &plan.operations,
            ApplyMode::Atomic,
            AutomationEvaluation::Skip,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|error| StorageError::Database(format!("failed to commit import: {error}")))?;
        Ok(plan.report)
    }

    /// Load the live rows and file-referenced tombstones the planner needs,
    /// inside the import's own transaction.
    async fn load_import_state(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        file: &SnapshotFile,
    ) -> Result<snapshot::ImportState, ServiceError> {
        let projects = self.store.get_all_projects(Some(&mut **tx)).await?;
        let mut sections = Vec::new();
        let mut tasks = Vec::new();
        let mut notes = Vec::new();
        let mut automations = Vec::new();

        for project in &projects {
            sections.extend(
                self.store
                    .get_all_sections_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            tasks.extend(
                self.store
                    .get_all_tasks_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            notes.extend(
                self.store
                    .get_all_notes_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            automations.extend(
                self.store
                    .get_all_automations_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
        }

        let mut referenced = Vec::new();
        for document in &file.projects {
            referenced.push((EntityType::Project, document.project.id));
            referenced.extend(
                document
                    .sections
                    .iter()
                    .map(|section| (EntityType::Section, section.id)),
            );
            referenced.extend(
                document
                    .tasks
                    .iter()
                    .map(|task| (EntityType::Task, task.id)),
            );
            referenced.extend(
                document
                    .notes
                    .iter()
                    .map(|note| (EntityType::Note, note.id)),
            );
            referenced.extend(
                document
                    .automations
                    .iter()
                    .map(|automation| (EntityType::Automation, automation.id)),
            );
        }

        let mut tombstones = HashSet::new();
        for (entity_type, id) in referenced {
            if self
                .store
                .is_tombstoned(entity_type, id, Some(&mut **tx))
                .await?
            {
                tombstones.insert((entity_type, id));
            }
        }

        Ok(snapshot::ImportState {
            projects,
            sections,
            tasks,
            notes,
            automations,
            tombstones,
        })
    }

    /// Apply a batch of operations in one transaction. `Atomic` rolls the
    /// batch back on the first rejection so one HTTP write cannot leave partial
    /// state; `PerOperation` acknowledges the rejection and applies the rest
    /// (#104's `/sync`).
    pub async fn apply_operations(
        &self,
        ops: &[Operation],
        mode: ApplyMode,
    ) -> Result<ApplyOutcome, ServiceError> {
        if ops.is_empty() {
            return Ok(ApplyOutcome::default());
        }

        let mut tx = self.store.begin_apply().await?;
        let outcome = self
            .apply_batch(&mut tx, ops, mode, AutomationEvaluation::Evaluate)
            .await?;
        tx.commit().await.map_err(|error| {
            StorageError::Database(format!("failed to commit apply batch: {error}"))
        })?;
        Ok(outcome)
    }

    async fn apply_batch(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        ops: &[Operation],
        mode: ApplyMode,
        automation_evaluation: AutomationEvaluation,
    ) -> Result<ApplyOutcome, ServiceError> {
        let mut acks = Vec::with_capacity(ops.len());
        let mut last_seq = None;

        for op in ops {
            let (ack, seq) = self
                .apply_one_operation(tx, op, automation_evaluation)
                .await?;
            if matches!(mode, ApplyMode::Atomic)
                && let AcknowledgmentStatus::Rejected { reason } = &ack.status
            {
                // dropping `tx` rolls the whole batch back
                return Err(rejection_error(op.operation_id, *reason));
            }
            last_seq = last_seq.max(seq);
            acks.push(ack);
        }

        Ok(ApplyOutcome { acks, last_seq })
    }

    /// Apply a pushed batch per-op, then return every change after the
    /// client's cursor plus the cursor to send next.
    ///
    /// `next_seq` is the last change returned, or `last_seen_seq` when the
    /// client is already up to date. No paging: the caller gets the whole
    /// remainder of the changes log.
    pub async fn sync(
        &self,
        ops: &[Operation],
        last_seen_seq: i64,
    ) -> Result<SyncResult, ServiceError> {
        let outcome = self.apply_operations(ops, ApplyMode::PerOperation).await?;
        let changes = self.store.changes_after(last_seen_seq, None).await?;
        let next_seq = changes
            .last()
            .map(|change| change.seq)
            .unwrap_or(last_seen_seq);

        Ok(SyncResult {
            acks: outcome.acks,
            changes,
            next_seq,
        })
    }

    /// The highest changes-log `seq`, or `0` when the log is empty.
    pub async fn max_seq(&self) -> Result<i64, ServiceError> {
        Ok(self.store.last_change_seq(None).await?)
    }

    async fn apply_one_operation(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        automation_evaluation: AutomationEvaluation,
    ) -> Result<(OperationAcknowledgment, Option<i64>), ServiceError> {
        // 1. an op whose wire version this build cannot read is rejected on its
        // own, so the rest of the batch still applies
        if op.validate().is_err() {
            return Ok((
                OperationAcknowledgment::rejected(
                    op.operation_id,
                    RejectionReason::UnsupportedVersion,
                ),
                None,
            ));
        }

        // 2. a re-transmission: acknowledge it without a second effect
        if self
            .store
            .is_operation_applied(op.operation_id, Some(&mut **tx))
            .await?
        {
            return Ok((OperationAcknowledgment::applied(op.operation_id), None));
        }

        // 3. an edit to a deleted entity loses; deletes are permanent
        let subject = subject(&op.kind);
        if self
            .store
            .is_tombstoned(subject.entity_type(), subject.id(), Some(&mut **tx))
            .await?
        {
            return Ok((
                OperationAcknowledgment::rejected(op.operation_id, RejectionReason::Tombstoned),
                None,
            ));
        }

        // 4-6. apply the fields the op names, write through the store, then log
        // one change per affected entity; deletes fan out via affected_entities
        let mut task_transition: Option<(Option<Task>, Task)> = None;
        let mut seq = match &op.kind {
            OperationKind::CreateProject {
                id,
                name,
                description,
                task_sorting_mode,
                show_priority,
                created_at,
            } => {
                if self
                    .store
                    .get_project_by_name(name, Some(&mut **tx))
                    .await?
                    .is_some()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
                }

                let project = Project {
                    id: *id,
                    name: name.clone(),
                    description: description
                        .clone()
                        .filter(|description| !description.is_empty()),
                    task_sorting_mode: *task_sorting_mode,
                    show_priority: *show_priority,
                    created_at: *created_at,
                };
                self.store
                    .create_project(project.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&project).expect("a project serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Project(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateProject { id, .. } => {
                let Some(mut project) = self.store.get_project_by_id(*id, Some(&mut **tx)).await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let field_values = project_field_updates(&mut project, &op.kind);
                self.store.update_project(project, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Project(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteProject { id } => {
                let existed = self
                    .store
                    .get_project_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Project(*id)).await?;
                let deleted_at = Utc::now();

                // children come last, so reversing logs them ahead of their parent
                let mut seqs = Vec::with_capacity(entities.len());
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

                if existed {
                    self.store
                        .delete_project_by_id(*id, Some(&mut **tx))
                        .await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateSection {
                id,
                project_id,
                name,
                position,
                color,
                created_at,
            } => {
                if self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .get_section_by_name(*project_id, name, Some(&mut **tx))
                    .await?
                    .is_some()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
                }

                let section = Section {
                    id: *id,
                    project_id: *project_id,
                    name: name.clone(),
                    position: position.clone(),
                    color: *color,
                    created_at: *created_at,
                };
                self.store
                    .create_section(section.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&section).expect("a section serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateSection { id, .. } => {
                let Some(mut section) = self.store.get_section_by_id(*id, Some(&mut **tx)).await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let field_values = section_field_updates(&mut section, &op.kind);
                self.store.update_section(section, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::MoveSection { id, .. } => {
                let Some(mut section) = self.store.get_section_by_id(*id, Some(&mut **tx)).await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let field_values = section_field_updates(&mut section, &op.kind);
                self.store.update_section(section, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteSection { id } => {
                let section = self.store.get_section_by_id(*id, Some(&mut **tx)).await?;
                let mut seqs = Vec::new();

                // a section's tasks survive its deletion, so they are relocated
                // to No section and logged as updates rather than tombstoned
                if let Some(section) = &section {
                    let automations = self
                        .store
                        .get_all_automations_by_project_id(section.project_id, Some(&mut **tx))
                        .await?;
                    if automations
                        .iter()
                        .any(|automation| automation.references_section(section.id))
                    {
                        return Ok((
                            OperationAcknowledgment::rejected(
                                op.operation_id,
                                RejectionReason::Conflict,
                            ),
                            None,
                        ));
                    }

                    let tasks = self
                        .store
                        .get_all_tasks_by_section_id(section.id, Some(&mut **tx))
                        .await?;
                    let unsectioned = self
                        .store
                        .get_all_tasks_by_project_id(section.project_id, Some(&mut **tx))
                        .await?
                        .into_iter()
                        .filter(|task| task.section_id.is_none())
                        .collect::<Vec<_>>();
                    let last = unsectioned.iter().map(|task| &task.position).max().cloned();
                    let positions =
                        n_keys_between(last.as_ref(), None, tasks.len()).map_err(position_error)?;

                    for (mut task, position) in tasks.into_iter().zip(positions) {
                        task.section_id = None;
                        task.position = position.clone();
                        let task_id = task.id;
                        self.store.update_task(task, Some(&mut **tx)).await?;

                        let mut field_values = Map::new();
                        field_values.insert("section_id".into(), Value::Null);
                        field_values.insert("position".into(), json!(position));
                        seqs.push(
                            self.record_change(
                                tx,
                                op,
                                Entity::Task(task_id),
                                ChangeKind::Update,
                                Value::Object(field_values),
                            )
                            .await?,
                        );
                    }
                }

                let entities = self.affected_entities(tx, Entity::Section(*id)).await?;
                let deleted_at = Utc::now();
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

                if section.is_some() {
                    self.store.delete_section(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateTask {
                id,
                project_id,
                title,
                description,
                priority,
                section_id,
                position,
                state,
                tags,
                created_at,
                external_identity,
                external_metadata,
            } => {
                if self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }
                if let Some(section_id) = section_id {
                    let belongs = self
                        .store
                        .get_section_by_id(*section_id, Some(&mut **tx))
                        .await?
                        .is_some_and(|section| section.project_id == *project_id);
                    let tombstoned = self
                        .store
                        .is_tombstoned(EntityType::Section, *section_id, Some(&mut **tx))
                        .await?;
                    if !belongs || tombstoned {
                        return Ok((
                            OperationAcknowledgment::rejected(
                                op.operation_id,
                                RejectionReason::NotFound,
                            ),
                            None,
                        ));
                    }
                }

                let task = Task {
                    id: *id,
                    project_id: *project_id,
                    title: title.clone(),
                    description: description
                        .clone()
                        .filter(|description| !description.is_empty()),
                    priority: *priority,
                    section_id: *section_id,
                    position: position.clone(),
                    state: *state,
                    tags: tags.clone(),
                    created_at: *created_at,
                    external_identity: external_identity.clone(),
                    external_metadata: external_metadata.as_deref().cloned(),
                };
                task_transition = Some((None, task.clone()));
                self.store
                    .create_task(task.clone(), Some(&mut **tx))
                    .await?;

                let field_values = serde_json::to_value(&task).expect("a task serializes to JSON");
                Some(
                    self.record_change(tx, op, Entity::Task(*id), ChangeKind::Create, field_values)
                        .await?,
                )
            }
            OperationKind::UpdateTask { id, .. } | OperationKind::MoveTask { id, .. } => {
                let Some(mut task) = self.store.get_task_by_id(*id, Some(&mut **tx)).await? else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let before = task.clone();
                let field_values = task_field_updates(&mut task, &op.kind);
                task_transition = Some((Some(before), task.clone()));
                self.store.update_task(task, Some(&mut **tx)).await?;
                Some(
                    self.record_change(tx, op, Entity::Task(*id), ChangeKind::Update, field_values)
                        .await?,
                )
            }
            OperationKind::DeleteTask { id } => {
                let existed = self
                    .store
                    .get_task_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Task(*id)).await?;
                let deleted_at = Utc::now();

                let mut seqs = Vec::with_capacity(entities.len());
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

                if existed {
                    self.store.delete_task(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateNote {
                id,
                task_id,
                contents,
                created_at,
            } => {
                if self
                    .store
                    .get_task_by_id(*task_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Task, *task_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }

                let note = Note {
                    id: *id,
                    task_id: *task_id,
                    contents: contents.clone(),
                    created_at: *created_at,
                };
                self.store
                    .create_note(note.clone(), Some(&mut **tx))
                    .await?;

                let field_values = serde_json::to_value(&note).expect("a note serializes to JSON");
                Some(
                    self.record_change(tx, op, Entity::Note(*id), ChangeKind::Create, field_values)
                        .await?,
                )
            }
            OperationKind::UpdateNote { id, .. } => {
                let Some(mut note) = self.store.get_note_by_id(*id, Some(&mut **tx)).await? else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let field_values = note_field_updates(&mut note, &op.kind);
                self.store.update_note(note, Some(&mut **tx)).await?;
                Some(
                    self.record_change(tx, op, Entity::Note(*id), ChangeKind::Update, field_values)
                        .await?,
                )
            }
            OperationKind::DeleteNote { id } => {
                let existed = self
                    .store
                    .get_note_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Note(*id)).await?;
                let deleted_at = Utc::now();

                let mut seqs = Vec::with_capacity(entities.len());
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

                if existed {
                    self.store.delete_note(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateAutomation {
                id,
                project_id,
                trigger,
                actions,
                created_at,
            } => {
                let Some(project) = self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }
                if let Some(reason) = self
                    .validate_automation(&project, trigger, actions, None, tx)
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(op.operation_id, reason),
                        None,
                    ));
                }

                let automation = Automation {
                    id: *id,
                    project_id: *project_id,
                    trigger: trigger.clone(),
                    actions: actions.clone(),
                    created_at: *created_at,
                };
                self.store
                    .create_automation(automation.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&automation).expect("an automation serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Automation(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateAutomation { id, actions } => {
                let Some(mut automation) = self
                    .store
                    .get_automation_by_id(*id, Some(&mut **tx))
                    .await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                let Some(project) = self
                    .store
                    .get_project_by_id(automation.project_id, Some(&mut **tx))
                    .await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                if let Some(reason) = self
                    .validate_automation(&project, &automation.trigger, actions, Some(*id), tx)
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(op.operation_id, reason),
                        None,
                    ));
                }

                automation.actions = actions.clone();
                self.store
                    .update_automation(automation, Some(&mut **tx))
                    .await?;

                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Automation(*id),
                        ChangeKind::Update,
                        json!({ "actions": actions }),
                    )
                    .await?,
                )
            }
            OperationKind::DeleteAutomation { id } => {
                let existed = self
                    .store
                    .get_automation_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Automation(*id)).await?;
                let deleted_at = Utc::now();

                let mut seqs = Vec::with_capacity(entities.len());
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

                if existed {
                    self.store.delete_automation(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
        };

        // 7. the effect is in the log, so a later replay is stopped at step 2
        self.store
            .record_applied_operation(op.operation_id, Utc::now(), Some(&mut **tx))
            .await?;

        if automation_evaluation == AutomationEvaluation::Evaluate
            && let Some((before, after)) = &task_transition
        {
            seq = seq.max(
                self.apply_matching_automations(tx, op, before.as_ref(), after)
                    .await?,
            );
        }

        Ok((OperationAcknowledgment::applied(op.operation_id), seq))
    }

    async fn affected_entities(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        entity: Entity,
    ) -> Result<Vec<Entity>, ServiceError> {
        let mut entities = vec![entity];

        match entity {
            Entity::Project(project_id) => {
                let sections = self
                    .store
                    .get_all_sections_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let tasks = self
                    .store
                    .get_all_tasks_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let notes = self
                    .store
                    .get_all_notes_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let automations = self
                    .store
                    .get_all_automations_by_project_id(project_id, Some(&mut **tx))
                    .await?;

                entities.extend(
                    sections
                        .into_iter()
                        .map(|section| Entity::Section(section.id)),
                );
                entities.extend(tasks.into_iter().map(|task| Entity::Task(task.id)));
                entities.extend(notes.into_iter().map(|note| Entity::Note(note.id)));
                entities.extend(
                    automations
                        .into_iter()
                        .map(|automation| Entity::Automation(automation.id)),
                );
            }
            Entity::Task(task_id) => {
                let notes = self
                    .store
                    .get_all_notes_by_task_id(task_id, Some(&mut **tx))
                    .await?;
                entities.extend(notes.into_iter().map(|note| Entity::Note(note.id)));
            }
            // a section's tasks survive and are relocated instead of tombstoned
            Entity::Section(_) | Entity::Note(_) | Entity::Automation(_) => {}
        }

        Ok(entities)
    }

    async fn record_change(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        entity: Entity,
        kind: ChangeKind,
        field_values: FieldValues,
    ) -> Result<i64, ServiceError> {
        let change = Change {
            // assigned by the database; the store ignores this placeholder
            seq: 0,
            entity_type: entity.entity_type(),
            entity_id: entity.id(),
            kind,
            operation_id: op.operation_id,
            client_timestamp: op.timestamp,
            field_values,
            created_at: Utc::now(),
        };

        Ok(self.store.record_change(change, Some(&mut **tx)).await?)
    }

    async fn resolve_section(
        &self,
        project: &Project,
        input: SectionInput,
    ) -> Result<(Option<Section>, Option<Operation>), ServiceError> {
        match input {
            SectionInput::Unchanged | SectionInput::NoSection => Ok((None, None)),
            SectionInput::Existing(section_id) => Ok((
                Some(self.require_section_in_project(project, section_id).await?),
                None,
            )),
            SectionInput::New(name) => {
                let (section, operation) = self.build_section(project, name, None).await?;
                Ok((Some(section), Some(operation)))
            }
        }
    }

    async fn validated_section_name(
        &self,
        project: &Project,
        exclude: Option<SectionId>,
        name: String,
    ) -> Result<String, ServiceError> {
        let name = name.to_string();
        if name.is_empty() {
            return Err(ServiceError::SectionNameRequired);
        }
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        if sections
            .iter()
            .any(|section| Some(section.id) != exclude && section.name == name)
        {
            return Err(ServiceError::SectionNameTaken { section: name });
        }
        Ok(name)
    }

    async fn tasks_in_section(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(match section_id {
            Some(section_id) => {
                self.store
                    .get_all_tasks_by_section_id(section_id, None)
                    .await?
            }
            None => self
                .store
                .get_all_tasks_by_project_id(project_id, None)
                .await?
                .into_iter()
                .filter(|task| task.section_id.is_none())
                .collect(),
        })
    }

    async fn last_task_position(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Option<Position>, ServiceError> {
        let tasks = self.tasks_in_section(project_id, section_id).await?;
        Ok(tasks.iter().map(|task| &task.position).max().cloned())
    }

    async fn append_position(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Position, ServiceError> {
        let last = self.last_task_position(project_id, section_id).await?;
        key_between(last.as_ref(), None).map_err(position_error)
    }

    /// The position that appends a task to the section or unsectioned group,
    /// computed inside the caller's transaction.
    async fn append_task_position_in_transaction(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Position, ServiceError> {
        let tasks = match section_id {
            Some(section_id) => {
                self.store
                    .get_all_tasks_by_section_id(section_id, Some(&mut **tx))
                    .await?
            }
            None => self
                .store
                .get_all_tasks_by_project_id(project_id, Some(&mut **tx))
                .await?
                .into_iter()
                .filter(|task| task.section_id.is_none())
                .collect(),
        };
        let last = tasks.iter().map(|task| &task.position).max().cloned();
        key_between(last.as_ref(), None).map_err(position_error)
    }

    /// Evaluate the project's rule for the trigger represented by `op`, then
    /// apply that rule's actions in order. Further automation evaluation is
    /// skipped for the generated actions.
    async fn apply_matching_automations(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        before: Option<&Task>,
        after: &Task,
    ) -> Result<Option<i64>, ServiceError> {
        let Some(trigger) = automation_trigger_for_task_change(&op.kind, before, after) else {
            return Ok(None);
        };

        let automations = self
            .store
            .get_all_automations_by_project_id(after.project_id, Some(&mut **tx))
            .await?;
        let Some(rule) = automations.iter().find(|rule| rule.trigger == trigger) else {
            return Ok(None);
        };

        let mut current = after.clone();
        let mut max_seq = None;

        for action in &rule.actions {
            if create_placement_blocks_move_action(&trigger, action, &op.kind) {
                continue;
            }
            if automation_action_is_noop(action, &current) {
                continue;
            }

            let kind = match action {
                AutomationAction::MoveToSection { section_id } => {
                    let position = self
                        .append_task_position_in_transaction(tx, current.project_id, *section_id)
                        .await?;
                    OperationKind::MoveTask {
                        id: current.id,
                        section_id: *section_id,
                        position,
                    }
                }
                AutomationAction::CloseTask => {
                    update_task_state_operation(current.id, TaskState::Closed)
                }
                AutomationAction::OpenTask => {
                    update_task_state_operation(current.id, TaskState::Open)
                }
                AutomationAction::SetPriority { priority } => {
                    update_task_priority_operation(current.id, *priority)
                }
            };

            let action_op = new_operation(Uuid::new_v4(), kind);
            // Box the recursive call: the engine suppresses further automation
            // evaluation at runtime, but the compiler still needs a sized future.
            let (ack, seq) =
                Box::pin(self.apply_one_operation(tx, &action_op, AutomationEvaluation::Skip))
                    .await?;
            max_seq = max_seq.max(seq);

            if ack.status == AcknowledgmentStatus::Applied
                && let Some(task) = self
                    .store
                    .get_task_by_id(current.id, Some(&mut **tx))
                    .await?
            {
                current = task;
            }
        }

        Ok(max_seq)
    }

    async fn require_project(&self, id: ProjectId) -> Result<Project, ServiceError> {
        self.store
            .get_project_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("project {id} not found")))
            })
    }

    async fn require_task(&self, id: TaskId) -> Result<Task, ServiceError> {
        self.store.get_task_by_id(id, None).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("task {id} not found")))
        })
    }

    async fn require_note(&self, id: NoteId) -> Result<Note, ServiceError> {
        self.store.get_note_by_id(id, None).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("note {id} not found")))
        })
    }

    async fn require_section(&self, id: SectionId) -> Result<Section, ServiceError> {
        self.store
            .get_section_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("section {id} not found")))
            })
    }

    async fn require_task_in_project(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<Task, ServiceError> {
        self.store
            .get_task_by_id(id, None)
            .await?
            .filter(|task| task.project_id == project.id)
            .ok_or_else(|| ServiceError::TaskNotFound {
                task_id: id,
                project: project.name.clone(),
            })
    }

    async fn require_section_in_project(
        &self,
        project: &Project,
        id: SectionId,
    ) -> Result<Section, ServiceError> {
        self.store
            .get_section_by_id(id, None)
            .await?
            .filter(|section| section.project_id == project.id)
            .ok_or_else(|| ServiceError::SectionNotFound {
                section: id.to_string(),
                project: project.name.clone(),
            })
    }
}

fn subject(op: &OperationKind) -> Entity {
    match op {
        OperationKind::CreateProject { id, .. }
        | OperationKind::UpdateProject { id, .. }
        | OperationKind::DeleteProject { id } => Entity::Project(*id),
        OperationKind::CreateSection { id, .. }
        | OperationKind::UpdateSection { id, .. }
        | OperationKind::MoveSection { id, .. }
        | OperationKind::DeleteSection { id } => Entity::Section(*id),
        OperationKind::CreateTask { id, .. }
        | OperationKind::UpdateTask { id, .. }
        | OperationKind::MoveTask { id, .. }
        | OperationKind::DeleteTask { id } => Entity::Task(*id),
        OperationKind::CreateNote { id, .. }
        | OperationKind::UpdateNote { id, .. }
        | OperationKind::DeleteNote { id } => Entity::Note(*id),
        OperationKind::CreateAutomation { id, .. }
        | OperationKind::UpdateAutomation { id, .. }
        | OperationKind::DeleteAutomation { id } => Entity::Automation(*id),
    }
}

fn task_field_updates(task: &mut Task, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    match op {
        OperationKind::UpdateTask {
            title,
            description,
            priority,
            tags,
            state,
            ..
        } => {
            if let Some(title) = title {
                task.title = title.clone();
                fields.insert("title".into(), json!(title));
            }
            if let Some(description) = description {
                task.description = description.clone();
                fields.insert("description".into(), json!(description));
            }
            if let Some(priority) = priority {
                task.priority = *priority;
                fields.insert("priority".into(), json!(priority));
            }
            if let Some(tags) = tags {
                task.tags = tags.clone();
                fields.insert("tags".into(), json!(tags));
            }
            if let Some(state) = state {
                task.state = *state;
                fields.insert("state".into(), json!(state));
            }
        }
        OperationKind::MoveTask {
            section_id,
            position,
            ..
        } => {
            task.section_id = *section_id;
            task.position = position.clone();
            fields.insert("section_id".into(), json!(section_id));
            fields.insert("position".into(), json!(position));
        }
        // the caller only passes a task op; anything else leaves it untouched
        _ => {}
    }

    Value::Object(fields)
}

/// The automation trigger represented by an applied task operation, if any.
///
/// `before` is the task before the operation and `after` is the task after it.
/// Creation fires only `TaskCreated`; state and section triggers require a
/// real change.
fn automation_trigger_for_task_change(
    op: &OperationKind,
    before: Option<&Task>,
    after: &Task,
) -> Option<AutomationTrigger> {
    match op {
        OperationKind::CreateTask { .. } => Some(AutomationTrigger::TaskCreated),
        OperationKind::UpdateTask { state: Some(_), .. } => {
            let before = before?;
            if before.state == after.state {
                None
            } else {
                Some(match after.state {
                    TaskState::Closed => AutomationTrigger::TaskClosed,
                    TaskState::Open => AutomationTrigger::TaskOpened,
                })
            }
        }
        OperationKind::MoveTask { .. } => {
            let before = before?;
            if before.section_id == after.section_id {
                None
            } else {
                after
                    .section_id
                    .map(|section_id| AutomationTrigger::TaskEntersSection { section_id })
            }
        }
        _ => None,
    }
}

/// Whether applying the action would leave the task unchanged.
fn automation_action_is_noop(action: &AutomationAction, task: &Task) -> bool {
    match action {
        AutomationAction::MoveToSection { section_id } => task.section_id == *section_id,
        AutomationAction::CloseTask => task.state == TaskState::Closed,
        AutomationAction::OpenTask => task.state == TaskState::Open,
        AutomationAction::SetPriority { priority } => task.priority == *priority,
    }
}

fn update_task_state_operation(id: TaskId, state: TaskState) -> OperationKind {
    OperationKind::UpdateTask {
        id,
        title: None,
        description: None,
        priority: None,
        tags: None,
        state: Some(state),
    }
}

fn update_task_priority_operation(id: TaskId, priority: Priority) -> OperationKind {
    OperationKind::UpdateTask {
        id,
        title: None,
        description: None,
        priority: Some(priority),
        tags: None,
        state: None,
    }
}

/// A task-created rule that moves the task is skipped when the create
/// explicitly placed it in a section.
fn create_placement_blocks_move_action(
    trigger: &AutomationTrigger,
    action: &AutomationAction,
    op: &OperationKind,
) -> bool {
    matches!(trigger, AutomationTrigger::TaskCreated)
        && matches!(action, AutomationAction::MoveToSection { .. })
        && matches!(
            op,
            OperationKind::CreateTask {
                section_id: Some(_),
                ..
            }
        )
}

fn project_field_updates(project: &mut Project, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    let OperationKind::UpdateProject {
        name,
        description,
        task_sorting_mode,
        show_priority,
        ..
    } = op
    else {
        return Value::Object(fields);
    };

    if let Some(name) = name {
        project.name = name.clone();
        fields.insert("name".into(), json!(name));
    }
    if let Some(description) = description {
        project.description = description.clone();
        fields.insert("description".into(), json!(description));
    }
    if let Some(task_sorting_mode) = task_sorting_mode {
        project.task_sorting_mode = *task_sorting_mode;
        fields.insert("task_sorting_mode".into(), json!(task_sorting_mode));
    }
    if let Some(show_priority) = show_priority {
        project.show_priority = *show_priority;
        fields.insert("show_priority".into(), json!(show_priority));
    }

    Value::Object(fields)
}

fn section_field_updates(section: &mut Section, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    match op {
        OperationKind::UpdateSection { name, color, .. } => {
            if let Some(name) = name {
                section.name = name.clone();
                fields.insert("name".into(), json!(name));
            }
            if let Some(color) = color {
                section.color = match color {
                    OperationColorChange::Clear => None,
                    OperationColorChange::Set(color) => Some(*color),
                };
                // a clear records an explicit null so a reader can tell removal from
                // "field unchanged"
                let value = match color {
                    OperationColorChange::Clear => Value::Null,
                    OperationColorChange::Set(color) => json!(color),
                };
                fields.insert("color".into(), value);
            }
        }
        OperationKind::MoveSection { position, .. } => {
            section.position = position.clone();
            fields.insert("position".into(), json!(position));
        }
        _ => {}
    }

    Value::Object(fields)
}

fn section_placement_position(
    sections: &[Section],
    target: &Section,
    placement: SectionPlacement,
) -> Result<Option<Position>, ServiceError> {
    let target_index = sections
        .iter()
        .position(|section| section.id == target.id)
        .ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!(
                "section {} missing from its project's ordered list",
                target.id
            )))
        })?;

    let others: Vec<&Section> = sections
        .iter()
        .filter(|section| section.id != target.id)
        .collect();

    match placement {
        SectionPlacement::Top => {
            if target_index == 0 {
                return Ok(None);
            }
            let first_other = others
                .first()
                .copied()
                .expect("a non-first section has at least one sibling");
            key_between(None, Some(&first_other.position))
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::Bottom => {
            if target_index == sections.len() - 1 {
                return Ok(None);
            }
            let last_other = others
                .last()
                .copied()
                .expect("a non-last section has at least one sibling");
            key_between(Some(&last_other.position), None)
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::Before(anchor_id) => {
            let anchor_index = sections
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            if anchor_index == target_index + 1 {
                return Ok(None);
            }
            let anchor = &sections[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let predecessor_position = others[..others_anchor_index]
                .iter()
                .rev()
                .copied()
                .find(|section| section.position < anchor.position)
                .map(|section| &section.position);
            key_between(predecessor_position, Some(&anchor.position))
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::After(anchor_id) => {
            let anchor_index = sections
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            if target_index == anchor_index + 1 {
                return Ok(None);
            }
            let anchor = &sections[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let successor_position = others[others_anchor_index + 1..]
                .iter()
                .copied()
                .find(|section| section.position > anchor.position)
                .map(|section| &section.position);
            key_between(Some(&anchor.position), successor_position)
                .map(Some)
                .map_err(position_error)
        }
    }
}

fn task_placement_position(
    tasks: &[Task],
    target: &Task,
    placement: TaskPlacement,
) -> Result<Option<Position>, ServiceError> {
    let target_index = tasks.iter().position(|task| task.id == target.id);

    let others: Vec<&Task> = tasks.iter().filter(|task| task.id != target.id).collect();

    match placement {
        TaskPlacement::Top => {
            if target_index == Some(0) {
                return Ok(None);
            }
            key_between(None, others.first().map(|task| &task.position))
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::Bottom => {
            if target_index.is_some_and(|index| index + 1 == tasks.len()) {
                return Ok(None);
            }
            key_between(others.last().map(|task| &task.position), None)
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::Before(anchor_id) => {
            if anchor_id == target.id {
                return Err(ServiceError::TaskMoveAnchorItself { task: target.id });
            }
            let anchor_index = tasks
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            if let Some(target_index) = target_index
                && anchor_index == target_index + 1
            {
                return Ok(None);
            }
            let anchor = &tasks[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let predecessor_position = others[..others_anchor_index]
                .iter()
                .rev()
                .copied()
                .find(|task| task.position < anchor.position)
                .map(|task| &task.position);
            key_between(predecessor_position, Some(&anchor.position))
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::After(anchor_id) => {
            if anchor_id == target.id {
                return Err(ServiceError::TaskMoveAnchorItself { task: target.id });
            }
            let anchor_index = tasks
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            if let Some(target_index) = target_index
                && target_index == anchor_index + 1
            {
                return Ok(None);
            }
            let anchor = &tasks[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let successor_position = others[others_anchor_index + 1..]
                .iter()
                .copied()
                .find(|task| task.position > anchor.position)
                .map(|task| &task.position);
            key_between(Some(&anchor.position), successor_position)
                .map(Some)
                .map_err(position_error)
        }
    }
}

fn note_field_updates(note: &mut Note, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    let OperationKind::UpdateNote { contents, .. } = op else {
        return Value::Object(fields);
    };

    note.contents = contents.clone();
    fields.insert("contents".into(), json!(contents));

    Value::Object(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    fn task(title: &str, description: Option<&str>) -> Task {
        Task {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            title: title.to_string(),
            description: description.map(String::from),
            priority: Priority::Medium,
            section_id: None,
            position: Position::parse("a0").expect("a0 is a valid position"),
            state: TaskState::Open,
            tags: Tags::from(vec!["alpha"]),
            created_at: Utc::now(),
            external_identity: None,
            external_metadata: None,
        }
    }

    fn task_at(id: TaskId, section_id: Option<SectionId>, position: &str) -> Task {
        Task {
            id,
            project_id: Uuid::new_v4(),
            title: "task".to_string(),
            description: None,
            priority: Priority::Medium,
            section_id,
            position: Position::parse(position).expect("valid position"),
            state: TaskState::Open,
            tags: Tags::default(),
            created_at: Utc::now(),
            external_identity: None,
            external_metadata: None,
        }
    }

    #[test]
    fn update_task_sets_only_the_fields_the_op_carries() {
        let mut task = task("old", Some("kept"));
        let op = OperationKind::UpdateTask {
            id: task.id,
            title: Some("new".into()),
            description: None,
            priority: None,
            tags: None,
            state: None,
        };

        let fields = task_field_updates(&mut task, &op);

        assert_eq!(task.title, "new");
        assert_eq!(task.description.as_deref(), Some("kept"));

        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields["title"], json!("new"));
        assert!(!fields.contains_key("description"));
    }

    #[test]
    fn later_update_replaces_the_earlier_value() {
        let mut task = task("old", None);
        let id = task.id;

        task_field_updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: Some("first".into()),
                description: None,
                priority: None,
                tags: None,
                state: None,
            },
        );
        let fields = task_field_updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: Some("second".into()),
                description: None,
                priority: None,
                tags: None,
                state: None,
            },
        );

        assert_eq!(task.title, "second");
        assert_eq!(fields["title"], json!("second"));
    }

    fn section(name: &str, position: &str) -> Section {
        Section {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            name: name.to_string(),
            position: Position::parse(position).expect("valid position"),
            color: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn top_places_a_section_before_the_first() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];

        let result = section_placement_position(&sections, target, SectionPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top should move a non-first section");
        };
        assert!(position < sections[0].position);
    }

    #[test]
    fn top_is_a_noop_for_the_first_section() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];

        let result = section_placement_position(&sections, target, SectionPlacement::Top);

        assert!(result.expect("top placement succeeds").is_none());
    }

    #[test]
    fn bottom_places_a_section_after_the_last() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];

        let result = section_placement_position(&sections, target, SectionPlacement::Bottom);

        let Some(position) = result.expect("bottom placement computes a position") else {
            panic!("bottom should move a non-last section");
        };
        assert!(position > sections[2].position);
    }

    #[test]
    fn bottom_is_a_noop_for_the_last_section() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];

        let result = section_placement_position(&sections, target, SectionPlacement::Bottom);

        assert!(result.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn top_and_bottom_are_noops_for_the_only_section() {
        let sections = vec![section("only", "a0")];
        let target = &sections[0];

        let top = section_placement_position(&sections, target, SectionPlacement::Top);
        let bottom = section_placement_position(&sections, target, SectionPlacement::Bottom);

        assert!(top.expect("top placement succeeds").is_none());
        assert!(bottom.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn before_places_a_section_between_the_left_neighbor_and_anchor() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];
        let anchor = sections[1].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent section");
        };
        assert!(position > sections[0].position);
        assert!(position < sections[1].position);
    }

    #[test]
    fn after_places_a_section_between_the_anchor_and_right_neighbor() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent section");
        };
        assert!(position > sections[1].position);
        assert!(position < sections[2].position);
    }

    #[test]
    fn before_is_a_noop_when_already_adjacent() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        assert!(result.expect("before placement succeeds").is_none());
    }

    #[test]
    fn after_is_a_noop_when_already_adjacent() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        assert!(result.expect("after placement succeeds").is_none());
    }

    #[test]
    fn before_skips_equal_position_neighbors() {
        let sections = vec![
            section("a0", "a0"),
            section("a1-left", "a1"),
            section("a1-right", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[3];
        let anchor = sections[2].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent section");
        };
        assert!(position > sections[0].position);
        assert!(position < sections[1].position);
    }

    #[test]
    fn after_skips_equal_position_neighbors() {
        let sections = vec![
            section("a0", "a0"),
            section("a1-left", "a1"),
            section("a1-right", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent section");
        };
        assert!(position > sections[1].position);
        assert!(position < sections[3].position);
    }

    #[test]
    fn before_with_an_unknown_anchor_is_not_found() {
        let sections = vec![section("a0", "a0"), section("a1", "a1")];
        let target = &sections[1];
        let anchor = Uuid::new_v4();

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        assert!(matches!(result, Err(ServiceError::SectionNotFound { .. })));
    }

    #[test]
    fn task_top_places_before_the_first() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];

        let result = task_placement_position(&tasks, target, TaskPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top should move a non-first task");
        };
        assert!(position < tasks[0].position);
    }

    #[test]
    fn task_top_is_a_noop_for_the_first_task() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];

        let result = task_placement_position(&tasks, target, TaskPlacement::Top);

        assert!(result.expect("top placement succeeds").is_none());
    }

    #[test]
    fn task_bottom_places_after_the_last() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];

        let result = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        let Some(position) = result.expect("bottom placement computes a position") else {
            panic!("bottom should move a non-last task");
        };
        assert!(position > tasks[2].position);
    }

    #[test]
    fn task_bottom_is_a_noop_for_the_last_task() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];

        let result = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        assert!(result.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn task_top_and_bottom_are_noops_for_the_only_task() {
        let tasks = vec![task_at(Uuid::new_v4(), None, "a0")];
        let target = &tasks[0];

        let top = task_placement_position(&tasks, target, TaskPlacement::Top);
        let bottom = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        assert!(top.expect("top placement succeeds").is_none());
        assert!(bottom.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn task_before_places_between_the_left_neighbor_and_anchor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent task");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_after_places_between_the_anchor_and_right_neighbor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent task");
        };
        assert!(position > tasks[1].position);
        assert!(position < tasks[2].position);
    }

    #[test]
    fn task_before_is_a_noop_when_already_adjacent() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(result.expect("before placement succeeds").is_none());
    }

    #[test]
    fn task_after_is_a_noop_when_already_adjacent() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        assert!(result.expect("after placement succeeds").is_none());
    }

    #[test]
    fn task_before_skips_equal_position_neighbors() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[3];
        let anchor = tasks[2].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent task");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_after_skips_equal_position_neighbors() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent task");
        };
        assert!(position > tasks[1].position);
        assert!(position < tasks[3].position);
    }

    #[test]
    fn task_top_into_a_different_group_is_not_a_noop() {
        let destination = Uuid::new_v4();
        let tasks = vec![
            task_at(Uuid::new_v4(), Some(destination), "a1"),
            task_at(Uuid::new_v4(), Some(destination), "a2"),
        ];
        let target = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");

        let result = task_placement_position(&tasks, &target, TaskPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top into a different group must not be a noop");
        };
        assert!(position < tasks[0].position);
    }

    #[test]
    fn task_before_into_a_different_group_places_before_the_anchor() {
        let destination = Uuid::new_v4();
        let tasks = vec![
            task_at(Uuid::new_v4(), Some(destination), "a1"),
            task_at(Uuid::new_v4(), Some(destination), "a2"),
        ];
        let target = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, &target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before into a different group must not be a noop");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_before_rejects_the_task_itself_as_anchor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
        ];
        let target = &tasks[1];
        let anchor = target.id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(matches!(
            result,
            Err(ServiceError::TaskMoveAnchorItself { .. })
        ));
    }

    #[test]
    fn task_before_with_an_unknown_anchor_is_not_found() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
        ];
        let target = &tasks[1];
        let anchor = Uuid::new_v4();

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(matches!(
            result,
            Err(ServiceError::Storage(StorageError::NotFound(_)))
        ));
    }

    #[test]
    fn automation_references_section_in_trigger_or_actions() {
        let section_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        let triggered = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskEntersSection { section_id },
            actions: vec![AutomationAction::CloseTask],
            created_at: Utc::now(),
        };
        assert!(triggered.references_section(section_id));
        assert!(!triggered.references_section(other_id));

        let actioned = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskCreated,
            actions: vec![AutomationAction::MoveToSection {
                section_id: Some(section_id),
            }],
            created_at: Utc::now(),
        };
        assert!(actioned.references_section(section_id));
        assert!(!actioned.references_section(other_id));

        let unrelated = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskClosed,
            actions: vec![AutomationAction::OpenTask],
            created_at: Utc::now(),
        };
        assert!(!unrelated.references_section(section_id));
    }

    #[test]
    fn automation_service_errors_have_the_expected_display_and_kind() {
        let id = Uuid::new_v4();
        let not_found = ServiceError::AutomationNotFound { id };
        assert_eq!(not_found.to_string(), format!("Automation {id} not found."));
        assert_eq!(AppError::Service(not_found).kind(), "not_found");

        let in_use = ServiceError::SectionInUseByAutomation {
            section: "Todo".to_string(),
        };
        assert_eq!(
            in_use.to_string(),
            "Section \"Todo\" is used by an automation. Remove or change that automation first."
        );
        assert_eq!(AppError::Service(in_use).kind(), "conflict");
    }

    fn create_task_op(task: &Task, section_id: Option<SectionId>) -> OperationKind {
        OperationKind::CreateTask {
            id: task.id,
            project_id: task.project_id,
            title: task.title.clone(),
            description: task.description.clone(),
            priority: task.priority,
            section_id,
            position: task.position.clone(),
            state: task.state,
            tags: task.tags.clone(),
            created_at: task.created_at,
            external_identity: task.external_identity.clone(),
            external_metadata: task.external_metadata.clone().map(Box::new),
        }
    }

    #[test]
    fn automation_trigger_for_task_change_detects_task_transitions() {
        let id = Uuid::new_v4();
        let section_id = Uuid::new_v4();
        let created = task_at(id, None, "a0");
        assert_eq!(
            automation_trigger_for_task_change(&create_task_op(&created, None), None, &created),
            Some(AutomationTrigger::TaskCreated)
        );

        let open = task_at(id, None, "a0");
        let mut closed = open.clone();
        closed.state = TaskState::Closed;
        let close_op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: Some(TaskState::Closed),
        };
        assert_eq!(
            automation_trigger_for_task_change(&close_op, Some(&open), &closed),
            Some(AutomationTrigger::TaskClosed)
        );

        let reopen_op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: Some(TaskState::Open),
        };
        assert_eq!(
            automation_trigger_for_task_change(&reopen_op, Some(&closed), &open),
            Some(AutomationTrigger::TaskOpened)
        );
        assert_eq!(
            automation_trigger_for_task_change(&reopen_op, Some(&open), &open),
            None
        );

        let unsectioned = task_at(id, None, "a0");
        let sectioned = task_at(id, Some(section_id), "a0");
        let enter_op = OperationKind::MoveTask {
            id,
            section_id: Some(section_id),
            position: sectioned.position.clone(),
        };
        assert_eq!(
            automation_trigger_for_task_change(&enter_op, Some(&unsectioned), &sectioned),
            Some(AutomationTrigger::TaskEntersSection { section_id })
        );
        assert_eq!(
            automation_trigger_for_task_change(&enter_op, Some(&sectioned), &sectioned),
            None
        );

        let leave_op = OperationKind::MoveTask {
            id,
            section_id: None,
            position: unsectioned.position.clone(),
        };
        assert_eq!(
            automation_trigger_for_task_change(&leave_op, Some(&sectioned), &unsectioned),
            None
        );

        let note_op = OperationKind::CreateNote {
            id: Uuid::new_v4(),
            task_id: id,
            contents: "note".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(
            automation_trigger_for_task_change(&note_op, None, &created),
            None
        );
    }

    #[test]
    fn automation_action_is_noop_matches_the_current_task() {
        let section_id = Uuid::new_v4();
        let task = task_at(Uuid::new_v4(), Some(section_id), "a0");
        assert!(automation_action_is_noop(
            &AutomationAction::MoveToSection {
                section_id: Some(section_id)
            },
            &task
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::MoveToSection { section_id: None },
            &task
        ));

        let unsectioned = task_at(Uuid::new_v4(), None, "a0");
        assert!(automation_action_is_noop(
            &AutomationAction::MoveToSection { section_id: None },
            &unsectioned
        ));

        assert!(!automation_action_is_noop(
            &AutomationAction::CloseTask,
            &task
        ));
        assert!(automation_action_is_noop(
            &AutomationAction::OpenTask,
            &task
        ));
        assert!(automation_action_is_noop(
            &AutomationAction::SetPriority {
                priority: Priority::Medium
            },
            &task
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::SetPriority {
                priority: Priority::High
            },
            &task
        ));

        let mut closed = task.clone();
        closed.state = TaskState::Closed;
        assert!(automation_action_is_noop(
            &AutomationAction::CloseTask,
            &closed
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::OpenTask,
            &closed
        ));
    }

    #[test]
    fn create_placement_blocks_move_action_only_for_explicit_create_sections() {
        let task = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");
        let section_id = task.section_id;
        let move_action = AutomationAction::MoveToSection { section_id };

        assert!(create_placement_blocks_move_action(
            &AutomationTrigger::TaskCreated,
            &move_action,
            &create_task_op(&task, section_id)
        ));
        assert!(!create_placement_blocks_move_action(
            &AutomationTrigger::TaskCreated,
            &move_action,
            &create_task_op(&task, None)
        ));
        assert!(!create_placement_blocks_move_action(
            &AutomationTrigger::TaskClosed,
            &move_action,
            &create_task_op(&task, section_id)
        ));
    }
}
