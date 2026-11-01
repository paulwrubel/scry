use std::collections::HashMap;

use crate::error::ServiceError;
use chrono::Utc;
use scry_core::backup::{self, Backup, ImportMode, ImportPlan, ImportReport};
use scry_core::error::StorageError;
use scry_core::ids::{
    Handle, MIN_HANDLE_LENGTH, NoteId, ProjectId, SectionId, TaskId, distinguishing_prefixes,
    shortest_handle, shortest_handles, template_section_id,
};
use scry_core::models::{
    CloseReason, Color, Lifecycle, Note, Priority, Project, ProjectTemplate, Section, Tags, Task,
    TaskSortingMode,
};
use scry_core::position::{Position, key_between, n_keys_between};
use scry_core::store::{ArcStore, ProjectSnapshot, ProjectToCreate, SectionToCreate, TaskToCreate};
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

/// Input for creating a task. `SectionInput::Unchanged` means no section.
#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
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

    pub async fn create_task(
        &self,
        project: &Project,
        input: NewTask,
    ) -> Result<TaskChangeResult, ServiceError> {
        let section = self.resolve_section(project, input.section).await?;
        let section_id = section.as_ref().map(|section| section.id);
        let position = self.append_position(project.id, section_id).await?;

        let task = self
            .store
            .create_task(TaskToCreate {
                id: Uuid::new_v4(),
                project_id: project.id,
                title: input.title,
                description: input
                    .description
                    .filter(|description| !description.is_empty()),
                priority: input.priority.unwrap_or_default(),
                section_id,
                position,
                tags: input.tags.unwrap_or_default(),
            })
            .await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn create_task_in_project(
        &self,
        project_id: ProjectId,
        input: NewTask,
    ) -> Result<TaskChangeResult, ServiceError> {
        let project = self.require_project(project_id).await?;
        self.create_task(&project, input).await
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

        let task = self
            .store
            .create_task(TaskToCreate {
                id: Uuid::new_v4(),
                project_id: source.project_id,
                title: source.title,
                description: source.description,
                priority: source.priority,
                section_id: source.section_id,
                position,
                tags: source.tags,
            })
            .await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn list_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_project_id(project_id).await?)
    }

    pub async fn list_tasks_by_section_id(
        &self,
        section_id: SectionId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_section_id(section_id).await?)
    }

    pub async fn update_task(
        &self,
        project: &Project,
        id: TaskId,
        patch: TaskPatch,
    ) -> Result<Task, ServiceError> {
        let mut task = self.require_task_in_project(project, id).await?;

        if patch.section != SectionInput::Unchanged {
            let section = self.resolve_section(project, patch.section).await?;
            let section_id = section.as_ref().map(|section| section.id);
            if section_id != task.section_id {
                task.position = self.append_position(project.id, section_id).await?;
                task.section_id = section_id;
            }
        }

        task.title = patch.title.unwrap_or(task.title);
        task.description = match patch.description {
            Some(description) => Some(description).filter(|description| !description.is_empty()),
            None => task.description,
        };
        task.priority = patch.priority.unwrap_or(task.priority);
        task.tags = patch.tags.unwrap_or(task.tags);

        Ok(self.store.update_task(task).await?)
    }

    pub async fn update_task_by_id(
        &self,
        task_id: TaskId,
        patch: TaskPatch,
    ) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.update_task(&project, task_id, patch).await
    }

    pub async fn move_task_to_section(
        &self,
        project: &Project,
        id: TaskId,
        section: SectionInput,
        position: Option<Position>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let mut task = self.require_task_in_project(project, id).await?;
        let unchanged = section == SectionInput::Unchanged;
        let section = if unchanged {
            match task.section_id {
                Some(section_id) => {
                    Some(self.require_section_in_project(project, section_id).await?)
                }
                None => None,
            }
        } else {
            self.resolve_section(project, section).await?
        };

        if unchanged && position.is_none() {
            return Ok(TaskChangeResult { task });
        }

        let section_id = section.as_ref().map(|section| section.id);
        task.position = match position {
            Some(position) => position,
            None => self.append_position(project.id, section_id).await?,
        };
        task.section_id = section_id;

        let task = self.store.update_task(task).await?;
        Ok(TaskChangeResult { task })
    }

    pub async fn move_task_to_section_by_id(
        &self,
        task_id: TaskId,
        section: SectionInput,
        position: Option<Position>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.move_task_to_section(&project, task_id, section, position)
            .await
    }

    pub async fn close_task(
        &self,
        project: &Project,
        id: TaskId,
        reason: CloseReason,
    ) -> Result<Task, ServiceError> {
        let mut task = self.require_task_in_project(project, id).await?;
        if task.lifecycle.reason() == Some(reason) {
            return Ok(task);
        }
        task.lifecycle = Lifecycle::Closed {
            closed_at: Utc::now(),
            reason,
        };
        Ok(self.store.update_task(task).await?)
    }

    pub async fn close_task_by_id(
        &self,
        task_id: TaskId,
        reason: CloseReason,
    ) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.close_task(&project, task_id, reason).await
    }

    pub async fn open_task(&self, project: &Project, id: TaskId) -> Result<Task, ServiceError> {
        let mut task = self.require_task_in_project(project, id).await?;
        if !task.lifecycle.is_closed() {
            return Ok(task);
        }
        task.lifecycle = Lifecycle::Open;
        Ok(self.store.update_task(task).await?)
    }

    pub async fn open_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.open_task(&project, task_id).await
    }

    pub async fn delete_task(&self, project: &Project, id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task_in_project(project, id).await?;
        self.store.delete_task(id).await?;
        Ok(task)
    }

    pub async fn delete_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.delete_task(&project, task_id).await
    }

    pub async fn add_note_to_task(
        &self,
        project: &Project,
        task_id: TaskId,
        contents: String,
    ) -> Result<Note, ServiceError> {
        self.require_task_in_project(project, task_id).await?;
        Ok(self
            .store
            .create_note(Uuid::new_v4(), task_id, contents)
            .await?)
    }

    pub async fn add_note(&self, task_id: TaskId, contents: String) -> Result<Note, ServiceError> {
        let task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        self.add_note_to_task(&project, task_id, contents).await
    }

    pub async fn list_notes_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, ServiceError> {
        Ok(self.store.get_all_notes_by_project_id(project_id).await?)
    }

    pub async fn list_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, ServiceError> {
        Ok(self.store.get_all_notes_by_task_id(task_id).await?)
    }

    pub async fn get_note(&self, id: NoteId) -> Result<Option<Note>, ServiceError> {
        Ok(self.store.get_note_by_id(id).await?)
    }

    pub async fn update_note(&self, note: Note) -> Result<Note, ServiceError> {
        Ok(self.store.update_note(note).await?)
    }

    pub async fn delete_note(&self, id: NoteId) -> Result<(), ServiceError> {
        self.store.delete_note(id).await?;
        Ok(())
    }

    // display handles are unique per entity type across the whole database,
    // including closed tasks and every project, so they need the complete id
    // set; display always starts at the 4-character floor
    pub async fn list_task_handles(&self) -> Result<HashMap<TaskId, String>, ServiceError> {
        let ids = self.store.get_all_task_ids().await?;
        Ok(shortest_handles(ids, MIN_HANDLE_LENGTH))
    }

    // display starts at the 4-character floor
    pub async fn get_task_handle(&self, id: TaskId) -> Result<String, ServiceError> {
        let ids = self.store.get_all_task_ids().await?;
        Ok(shortest_handle(id, ids, MIN_HANDLE_LENGTH))
    }

    pub async fn get_task_by_handle(&self, handle: &Handle) -> Result<Task, ServiceError> {
        let matches = self.store.find_task_ids_by_handle(handle, 2).await?;
        match matches.len() {
            0 => Err(ServiceError::TaskHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_task(matches[0]).await,
            _ => {
                let all = self.store.find_task_ids_by_handle(handle, i64::MAX).await?;
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
        let matches = self.store.find_note_ids_by_handle(handle, 2).await?;
        match matches.len() {
            0 => Err(ServiceError::NoteHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_note(matches[0]).await,
            _ => {
                let all = self.store.find_note_ids_by_handle(handle, i64::MAX).await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousNoteHandle {
                    handle: handle.to_string(),
                    candidates,
                })
            }
        }
    }

    pub async fn create_section(
        &self,
        project: &Project,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let name = self.validated_section_name(project, None, name).await?;
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id)
            .await?;
        let last = sections
            .iter()
            .map(|section| &section.position)
            .max()
            .cloned();
        let position = key_between(last.as_ref(), None).map_err(position_error)?;
        let id = Uuid::new_v4();
        Ok(self
            .store
            .create_section(SectionToCreate {
                id,
                project_id: project.id,
                name,
                position,
                color,
            })
            .await?)
    }

    pub async fn create_section_in_project(
        &self,
        project_id: ProjectId,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let project = self.require_project(project_id).await?;
        self.create_section(&project, name, color).await
    }

    /// Look up a section by its exact name within a project.
    pub async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Section, ServiceError> {
        match self.store.get_section_by_name(project_id, name).await? {
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
        Ok(self.store.get_section_by_id(id).await?)
    }

    pub async fn list_sections_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Section>, ServiceError> {
        Ok(self
            .store
            .get_all_sections_by_project_id(project_id)
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
        Ok(self
            .store
            .update_section(Section {
                name,
                color,
                ..section
            })
            .await?)
    }

    pub async fn delete_section(
        &self,
        project: &Project,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section_in_project(project, section_id).await?;
        self.store.delete_section(section.id).await?;
        Ok(section)
    }

    pub async fn delete_section_by_id(
        &self,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let project = self.require_project(section.project_id).await?;
        self.delete_section(&project, section_id).await
    }

    pub async fn merge_sections(
        &self,
        project: &Project,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.require_section_in_project(project, source_id).await?;
        let target = self.require_section_in_project(project, target_id).await?;
        if source.id == target.id {
            return Err(ServiceError::SectionMergeIntoItself {
                section: source.name.clone(),
            });
        }

        let source_tasks = self.tasks_in_section(project.id, Some(source.id)).await?;
        let last_target = self.last_task_position(project.id, Some(target.id)).await?;
        let positions = n_keys_between(last_target.as_ref(), None, source_tasks.len())
            .map_err(position_error)?;
        for (mut task, position) in source_tasks.into_iter().zip(positions) {
            task.section_id = Some(target.id);
            task.position = position;
            self.store.update_task(task).await?;
        }
        self.store.delete_section(source.id).await?;

        Ok((source, target))
    }

    pub async fn merge_sections_by_id(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.require_section(source_id).await?;
        let target = self.require_section(target_id).await?;
        let source_project = self.require_project(source.project_id).await?;
        self.require_project(target.project_id).await?;
        self.merge_sections(&source_project, source.id, target.id)
            .await
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

        let id = Uuid::new_v4();
        let project = self
            .store
            .create_project(ProjectToCreate {
                id,
                name,
                description: description.filter(|description| !description.is_empty()),
                task_sorting_mode,
                show_priority,
            })
            .await?;

        let Some(sections) = template_sections else {
            return Ok(project);
        };

        for section in sections {
            let position = Position::parse(section.position).map_err(position_error)?;
            let id = template_section_id(project.id, section.name);
            self.store
                .create_section(SectionToCreate {
                    id,
                    project_id: project.id,
                    name: section.name.to_string(),
                    position,
                    color: section.color,
                })
                .await?;
        }

        Ok(project)
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.store.get_all_projects().await?)
    }

    pub async fn get_project(&self, id: ProjectId) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_id(id).await?)
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_name(name).await?)
    }

    pub async fn project_snapshot(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectSnapshot, ServiceError> {
        Ok(self.store.get_project_snapshot(project_id).await?)
    }

    pub async fn update_project(
        &self,
        project: &Project,
        update: UpdateProject,
    ) -> Result<Project, ServiceError> {
        let description = match update.description {
            Some(value) => Some(value).filter(|value| !value.is_empty()),
            None => project.description.clone(),
        };
        Ok(self
            .store
            .update_project(Project {
                name: update.name.unwrap_or_else(|| project.name.clone()),
                description,
                task_sorting_mode: update
                    .task_sorting_mode
                    .unwrap_or(project.task_sorting_mode),
                show_priority: update.show_priority.unwrap_or(project.show_priority),
                ..project.clone()
            })
            .await?)
    }

    /// Delete a project by id, returning the deleted project.
    pub async fn delete_project_by_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Project, ServiceError> {
        let project = self.require_project(project_id).await?;
        self.store.delete_project_by_id(project.id).await?;
        Ok(project)
    }

    pub async fn export(&self, project: Option<ProjectId>) -> Result<Backup, ServiceError> {
        Ok(backup::export(self.store.as_ref(), project).await?)
    }

    pub async fn plan_import(
        &self,
        backup: &Backup,
        mode: ImportMode,
    ) -> Result<ImportPlan, ServiceError> {
        Ok(backup::plan_import(self.store.as_ref(), backup, mode).await?)
    }

    pub async fn import(&self, plan: ImportPlan) -> Result<ImportReport, ServiceError> {
        Ok(self.store.import(plan).await?)
    }

    async fn resolve_section(
        &self,
        project: &Project,
        input: SectionInput,
    ) -> Result<Option<Section>, ServiceError> {
        match input {
            SectionInput::Unchanged | SectionInput::NoSection => Ok(None),
            SectionInput::Existing(section_id) => Ok(Some(
                self.require_section_in_project(project, section_id).await?,
            )),
            SectionInput::New(name) => Ok(Some(self.create_section(project, name, None).await?)),
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
            .get_all_sections_by_project_id(project.id)
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
            Some(section_id) => self.store.get_all_tasks_by_section_id(section_id).await?,
            None => self
                .store
                .get_all_tasks_by_project_id(project_id)
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

    async fn require_project(&self, id: ProjectId) -> Result<Project, ServiceError> {
        self.store.get_project_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("project {id} not found")))
        })
    }

    async fn require_task(&self, id: TaskId) -> Result<Task, ServiceError> {
        self.store.get_task_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("task {id} not found")))
        })
    }

    async fn require_note(&self, id: NoteId) -> Result<Note, ServiceError> {
        self.store.get_note_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("note {id} not found")))
        })
    }

    async fn require_section(&self, id: SectionId) -> Result<Section, ServiceError> {
        self.store.get_section_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("section {id} not found")))
        })
    }

    async fn require_task_in_project(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<Task, ServiceError> {
        self.store
            .get_task_by_id(id)
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
            .get_section_by_id(id)
            .await?
            .filter(|section| section.project_id == project.id)
            .ok_or_else(|| ServiceError::SectionNotFound {
                section: id.to_string(),
                project: project.name.clone(),
            })
    }
}
