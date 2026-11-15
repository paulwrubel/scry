use crate::error::ServiceError;
use chrono::Utc;
use scry_core::error::StorageError;
use scry_core::ids::{
    AutomationId, CommentId, Handle, MIN_HANDLE_LENGTH, ProjectId, SectionId, TaskId,
    distinguishing_prefixes, shortest_handle, shortest_handles,
};
use scry_core::models::{
    Automation, AutomationAction, AutomationTrigger, Change, ChangeKind, Color, Comment, Entity,
    EntityType, ExternalIdentity, ExternalMetadata, FieldValues, Priority, Project, ProjectSnapshot,
    Section, Tags, Task, TaskState,
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
use sqlx::{Postgres, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

mod automation;
mod fields;
mod placement;
use self::fields::{
    comment_field_updates, project_field_updates, section_field_updates, subject,
    task_field_updates, update_changes_locked_external_field,
};
use self::placement::{section_placement_position, task_placement_position};

#[cfg(test)]
mod test_helpers;

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
    pub state: Option<TaskState>,
    pub external_identity: Option<Option<ExternalIdentity>>,
    pub external_metadata: Option<Option<ExternalMetadata>>,
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

        let created_at = Utc::now();
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
            created_at,
            updated_at: created_at,
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
                updated_at: None,
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

        let created_at = Utc::now();
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
            created_at,
            updated_at: created_at,
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
                updated_at: None,
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
                    updated_at: None,
                },
            ));
        }

        if patch.title.is_some()
            || patch.description.is_some()
            || patch.priority.is_some()
            || patch.tags.is_some()
            || patch.state.is_some()
            || patch.external_identity.is_some()
            || patch.external_metadata.is_some()
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
                state: patch.state,
                external_identity: patch.external_identity,
                external_metadata: patch
                    .external_metadata
                    .map(|metadata| metadata.map(Box::new)),
                updated_at: None,
            };
            if update_changes_locked_external_field(&task, &kind) {
                return Err(ServiceError::Storage(StorageError::Conflict(format!(
                    "task {} is linked to an external identity; its title, description, and state are locked",
                    task.id
                ))));
            }
            ops.push(new_operation(Uuid::new_v4(), kind));
        }

        self.apply_operations(&ops, ApplyMode::Atomic).await?;
        self.require_task(task_id).await
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
                updated_at: None,
            },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(TaskChangeResult {
            task: self.require_task(task_id).await?,
        })
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
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_task(task_id).await
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
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_task(task_id).await
    }

    pub async fn delete_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteTask { id: task_id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(task)
    }

    pub async fn add_comment(
        &self,
        task_id: TaskId,
        contents: String,
    ) -> Result<Comment, ServiceError> {
        self.require_task(task_id).await?;
        let created_at = Utc::now();
        let comment = Comment {
            id: Uuid::new_v4(),
            task_id,
            contents,
            created_at,
            updated_at: created_at,
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateComment {
                id: comment.id,
                task_id: comment.task_id,
                contents: comment.contents.clone(),
                created_at: comment.created_at,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(comment)
    }

    pub async fn list_comments_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Comment>, ServiceError> {
        Ok(self
            .store
            .get_all_comments_by_project_id(project_id, None)
            .await?)
    }

    pub async fn list_comments_by_task_id(
        &self,
        task_id: TaskId,
    ) -> Result<Vec<Comment>, ServiceError> {
        Ok(self
            .store
            .get_all_comments_by_task_id(task_id, None)
            .await?)
    }

    pub async fn get_comment(&self, id: CommentId) -> Result<Option<Comment>, ServiceError> {
        Ok(self.store.get_comment_by_id(id, None).await?)
    }

    pub async fn update_comment(&self, comment: Comment) -> Result<Comment, ServiceError> {
        self.require_comment(comment.id).await?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateComment {
                id: comment.id,
                contents: comment.contents.clone(),
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_comment(comment.id).await
    }

    pub async fn delete_comment(&self, id: CommentId) -> Result<(), ServiceError> {
        self.require_comment(id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteComment { id });
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

    // the current API has no comment-prefix entry point; this exists so comment
    // handles resolve like task handles wherever comment ids are later accepted
    #[allow(dead_code)]
    pub async fn get_comment_by_handle(&self, handle: &Handle) -> Result<Comment, ServiceError> {
        let matches = self
            .store
            .find_comment_ids_by_handle(handle, 2, None)
            .await?;
        match matches.len() {
            0 => Err(ServiceError::CommentHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_comment(matches[0]).await,
            _ => {
                let all = self
                    .store
                    .find_comment_ids_by_handle(handle, i64::MAX, None)
                    .await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousCommentHandle {
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
            updated_at: created_at,
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
                updated_at: None,
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
        let operation_color = match update.color {
            ColorChange::Clear => OperationColorChange::Clear,
            ColorChange::Set(color) => OperationColorChange::Set(color),
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateSection {
                id: section_id,
                name: Some(name),
                color: Some(operation_color),
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_section(section_id).await
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
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::MoveSection {
                id: section.id,
                position: new_position,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_section(section_id).await
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
                    updated_at: None,
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
        let created_at = Utc::now();
        let automation = Automation {
            id: Uuid::new_v4(),
            project_id: project.id,
            trigger,
            actions: actions.clone(),
            created_at,
            updated_at: created_at,
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateAutomation {
                id: automation.id,
                project_id: automation.project_id,
                trigger: automation.trigger.clone(),
                actions,
                created_at: automation.created_at,
                updated_at: None,
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
        self.store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateAutomation {
                id,
                actions,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })
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

    /// Create a project.
    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Project, ServiceError> {
        let created_at = Utc::now();
        let project = Project {
            id: Uuid::new_v4(),
            name,
            description: description.filter(|description| !description.is_empty()),
            created_at,
            updated_at: created_at,
        };

        if self.get_project_by_name(&project.name).await?.is_some() {
            return Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                project.name
            ))
            .into());
        }

        let ops = vec![new_operation(
            Uuid::new_v4(),
            OperationKind::CreateProject {
                id: project.id,
                name: project.name.clone(),
                description: project.description.clone(),
                created_at: project.created_at,
                updated_at: None,
            },
        )];

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
            ..project.clone()
        };

        let changed = update.name.is_some() || update.description.is_some();
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
                    updated_at: None,
                },
            );
            self.apply_operations(&[op], ApplyMode::Atomic).await?;
            return self.require_project(project.id).await;
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
        let mut comments = Vec::new();
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
            comments.extend(
                self.store
                    .get_all_comments_by_project_id(project.id, Some(&mut **tx))
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
                    .comments
                    .iter()
                    .map(|comment| (EntityType::Comment, comment.id)),
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
            comments,
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
                created_at,
                updated_at,
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
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
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
            OperationKind::UpdateProject { id, updated_at, .. } => {
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

                let mut field_values = project_field_updates(&mut project, &op.kind);
                project.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                field_values["updated_at"] = json!(project.updated_at);
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
                updated_at,
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
                    updated_at: (*updated_at).unwrap_or(*created_at),
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
            OperationKind::UpdateSection { id, updated_at, .. } => {
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

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                field_values["updated_at"] = json!(section.updated_at);
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
            OperationKind::MoveSection { id, updated_at, .. } => {
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

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                field_values["updated_at"] = json!(section.updated_at);
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

                    let relocated_at = Utc::now();
                    for (mut task, position) in tasks.into_iter().zip(positions) {
                        task.section_id = None;
                        task.position = position.clone();
                        task.updated_at = relocated_at;
                        let task_id = task.id;
                        self.store.update_task(task, Some(&mut **tx)).await?;

                        let mut field_values = Map::new();
                        field_values.insert("section_id".into(), Value::Null);
                        field_values.insert("position".into(), json!(position));
                        field_values.insert("updated_at".into(), json!(relocated_at));
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
                updated_at,
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
                    updated_at: (*updated_at).unwrap_or(*created_at),
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
            OperationKind::UpdateTask { id, updated_at, .. }
            | OperationKind::MoveTask { id, updated_at, .. } => {
                let Some(mut task) = self.store.get_task_by_id(*id, Some(&mut **tx)).await? else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                if matches!(&op.kind, OperationKind::UpdateTask { .. })
                    && update_changes_locked_external_field(&task, &op.kind)
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
                }

                let before = task.clone();
                let mut field_values = task_field_updates(&mut task, &op.kind);
                task.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                field_values["updated_at"] = json!(task.updated_at);
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
            OperationKind::CreateComment {
                id,
                task_id,
                contents,
                created_at,
                updated_at,
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

                let comment = Comment {
                    id: *id,
                    task_id: *task_id,
                    contents: contents.clone(),
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                };
                self.store
                    .create_comment(comment.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&comment).expect("a comment serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Comment(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateComment { id, updated_at, .. } => {
                let Some(mut comment) = self.store.get_comment_by_id(*id, Some(&mut **tx)).await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let mut field_values = comment_field_updates(&mut comment, &op.kind);
                comment.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                field_values["updated_at"] = json!(comment.updated_at);
                self.store.update_comment(comment, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Comment(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteComment { id } => {
                let existed = self
                    .store
                    .get_comment_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Comment(*id)).await?;
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
                    self.store.delete_comment(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateAutomation {
                id,
                project_id,
                trigger,
                actions,
                created_at,
                updated_at,
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
                    updated_at: (*updated_at).unwrap_or(*created_at),
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
            OperationKind::UpdateAutomation {
                id,
                actions,
                updated_at,
            } => {
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
                automation.updated_at = (*updated_at).unwrap_or_else(Utc::now);
                let field_values =
                    json!({ "actions": actions, "updated_at": automation.updated_at });
                self.store
                    .update_automation(automation, Some(&mut **tx))
                    .await?;

                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Automation(*id),
                        ChangeKind::Update,
                        field_values,
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
                let comments = self
                    .store
                    .get_all_comments_by_project_id(project_id, Some(&mut **tx))
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
                entities.extend(
                    comments
                        .into_iter()
                        .map(|comment| Entity::Comment(comment.id)),
                );
                entities.extend(
                    automations
                        .into_iter()
                        .map(|automation| Entity::Automation(automation.id)),
                );
            }
            Entity::Task(task_id) => {
                let comments = self
                    .store
                    .get_all_comments_by_task_id(task_id, Some(&mut **tx))
                    .await?;
                entities.extend(
                    comments
                        .into_iter()
                        .map(|comment| Entity::Comment(comment.id)),
                );
            }
            // a section's tasks survive and are relocated instead of tombstoned
            Entity::Section(_) | Entity::Comment(_) | Entity::Automation(_) => {}
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

    async fn require_comment(&self, id: CommentId) -> Result<Comment, ServiceError> {
        self.store
            .get_comment_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("comment {id} not found")))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn automation_references_section_in_trigger_or_actions() {
        let section_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let created_at = Utc::now();

        let triggered = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskEntersSection { section_id },
            actions: vec![AutomationAction::CloseTask],
            created_at,
            updated_at: created_at,
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
            created_at,
            updated_at: created_at,
        };
        assert!(actioned.references_section(section_id));
        assert!(!actioned.references_section(other_id));

        let unrelated = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskClosed,
            actions: vec![AutomationAction::OpenTask],
            created_at,
            updated_at: created_at,
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
}
