use crate::error::ServiceError;
use chrono::Utc;
use scry_core::backup::{self, Backup, ImportMode, ImportPlan, ImportReport};
use scry_core::error::StorageError;
use scry_core::models::{
    CloseReason, Color, Lifecycle, Note, NoteId, Priority, Project, ProjectId, ProjectTemplate,
    Section, SectionId, Tags, Task, TaskId, TaskSortingMode,
};
use scry_core::store::{ArcStore, ProjectSnapshot, TaskContentUpdate, TaskToCreate};

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

/// Partial task input, shared by create and update. `None` leaves a field
/// unchanged on update; `Some("")` on `description` clears it; `Some(Tags)` on
/// `tags` sets them (an empty `Tags` clears them). `title` is required when
/// creating a task. On create, `SectionInput::Unchanged` means no section.
#[derive(Debug, Clone, Default)]
pub struct TaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
}

/// A task plus the section it now lives in, so callers can name it without a
/// second lookup. `None` means the task has no section.
#[derive(Debug, Clone)]
pub struct TaskChangeResult {
    pub task: Task,
    pub section: Option<Section>,
}

/// Partial project input. `None` leaves a field unchanged; `Some("")` on
/// `description` clears it.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub task_sorting_mode: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
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
        input: TaskInput,
    ) -> Result<TaskChangeResult, ServiceError> {
        let title = input.title.ok_or(ServiceError::TitleRequired)?;
        let section = self.resolve_section(project, input.section).await?;
        let section_id = section.as_ref().map(|section| section.id);
        let position = self.next_position(project.id, section_id).await?;

        let task = self
            .store
            .create_task(TaskToCreate {
                project_id: project.id,
                title,
                description: input
                    .description
                    .filter(|description| !description.is_empty()),
                priority: input.priority.unwrap_or_default(),
                section_id,
                position,
                tags: input.tags.unwrap_or_default(),
            })
            .await?;

        Ok(TaskChangeResult { task, section })
    }

    pub async fn create_task_in_project(
        &self,
        project_id: ProjectId,
        input: TaskInput,
    ) -> Result<TaskChangeResult, ServiceError> {
        let project = self.project_by_id(project_id).await?;
        self.create_task(&project, input).await
    }

    pub async fn duplicate_task(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<TaskChangeResult, ServiceError> {
        self.task_in_project(project, id).await?;
        let task = self.store.duplicate_task(id).await?;
        let section = match task.section_id {
            Some(section_id) => Some(self.section_in_project(project, section_id).await?),
            None => None,
        };
        Ok(TaskChangeResult { task, section })
    }

    pub async fn list_tasks_by_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_project_id(project_id).await?)
    }

    pub async fn list_tasks_by_section(
        &self,
        section_id: SectionId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_section_id(section_id).await?)
    }

    pub async fn find_task(&self, id: TaskId) -> Result<Option<Task>, ServiceError> {
        Ok(self.store.get_task_by_id(id).await?)
    }

    pub async fn update_task(
        &self,
        project: &Project,
        id: TaskId,
        patch: TaskInput,
    ) -> Result<Task, ServiceError> {
        let mut task = self.task_in_project(project, id).await?;

        if patch.section != SectionInput::Unchanged {
            let section = self.resolve_section(project, patch.section).await?;
            let section_id = section.as_ref().map(|section| section.id);
            if section_id != task.section_id {
                let position = self.next_position(project.id, section_id).await?;
                task = self.store.move_task(task.id, section_id, position).await?;
            }
        }

        let description = match patch.description {
            Some(description) => Some(description).filter(|description| !description.is_empty()),
            None => task.description,
        };

        Ok(self
            .store
            .update_task(
                task.id,
                TaskContentUpdate {
                    title: patch.title.unwrap_or(task.title),
                    description,
                    priority: patch.priority.unwrap_or(task.priority),
                    tags: patch.tags.unwrap_or(task.tags),
                },
            )
            .await?)
    }

    pub async fn update_task_by_id(
        &self,
        task_id: TaskId,
        patch: TaskInput,
    ) -> Result<Task, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.update_task(&project, task_id, patch).await
    }

    pub async fn move_task(
        &self,
        project: &Project,
        id: TaskId,
        section: SectionInput,
        position: Option<i32>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let task = self.task_in_project(project, id).await?;
        let unchanged = section == SectionInput::Unchanged;
        let section = if unchanged {
            match task.section_id {
                Some(section_id) => Some(self.section_in_project(project, section_id).await?),
                None => None,
            }
        } else {
            self.resolve_section(project, section).await?
        };

        if unchanged && position.is_none() {
            return Ok(TaskChangeResult { task, section });
        }

        let section_id = section.as_ref().map(|section| section.id);
        let tasks = self.tasks_in_section(project.id, section_id).await?;
        let target_position = match position {
            Some(position) => position.clamp(0, tasks.len() as i32),
            None => tasks
                .iter()
                .map(|task| task.position)
                .max()
                .map_or(0, |position| position + 1),
        };

        let task = self
            .store
            .move_task(task.id, section_id, target_position)
            .await?;
        Ok(TaskChangeResult { task, section })
    }

    pub async fn move_task_by_id(
        &self,
        task_id: TaskId,
        section: SectionInput,
        position: Option<i32>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.move_task(&project, task_id, section, position).await
    }

    pub async fn close_task(
        &self,
        project: &Project,
        id: TaskId,
        reason: CloseReason,
    ) -> Result<Task, ServiceError> {
        let task = self.task_in_project(project, id).await?;
        if task.lifecycle.reason() == Some(reason) {
            return Ok(task);
        }
        Ok(self
            .store
            .set_lifecycle(
                task.id,
                Lifecycle::Closed {
                    closed_at: Utc::now(),
                    reason,
                },
            )
            .await?)
    }

    pub async fn close_task_by_id(
        &self,
        task_id: TaskId,
        reason: CloseReason,
    ) -> Result<Task, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.close_task(&project, task_id, reason).await
    }

    pub async fn open_task(&self, project: &Project, id: TaskId) -> Result<Task, ServiceError> {
        let task = self.task_in_project(project, id).await?;
        if !task.lifecycle.is_closed() {
            return Ok(task);
        }
        Ok(self.store.set_lifecycle(task.id, Lifecycle::Open).await?)
    }

    pub async fn open_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.open_task(&project, task_id).await
    }

    pub async fn delete_task(&self, project: &Project, id: TaskId) -> Result<Task, ServiceError> {
        let task = self.task_in_project(project, id).await?;
        self.store.delete_task(id).await?;
        Ok(task)
    }

    pub async fn delete_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.delete_task(&project, task_id).await
    }

    pub async fn add_task_note(
        &self,
        project: &Project,
        task_id: TaskId,
        contents: String,
    ) -> Result<Note, ServiceError> {
        self.task_in_project(project, task_id).await?;
        Ok(self.store.create_note(task_id, contents).await?)
    }

    pub async fn add_note(&self, task_id: TaskId, contents: String) -> Result<Note, ServiceError> {
        let task = self.task_by_id(task_id).await?;
        let project = self.project_by_id(task.project_id).await?;
        self.add_task_note(&project, task_id, contents).await
    }

    pub async fn list_notes_by_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, ServiceError> {
        Ok(self.store.get_all_notes_by_project_id(project_id).await?)
    }

    pub async fn list_notes_by_task(&self, task_id: TaskId) -> Result<Vec<Note>, ServiceError> {
        Ok(self.store.get_all_notes_by_task_id(task_id).await?)
    }

    pub async fn find_note(&self, id: NoteId) -> Result<Option<Note>, ServiceError> {
        Ok(self.store.get_note_by_id(id).await?)
    }

    pub async fn update_note(&self, note: Note) -> Result<Note, ServiceError> {
        Ok(self.store.update_note(note).await?)
    }

    pub async fn delete_note(&self, id: NoteId) -> Result<(), ServiceError> {
        self.store.delete_note(id).await?;
        Ok(())
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
        let position = sections
            .iter()
            .map(|section| section.position)
            .max()
            .map_or(0, |position| position + 1);
        Ok(self
            .store
            .create_section(project.id, name, position, color)
            .await?)
    }

    pub async fn create_section_in_project(
        &self,
        project_id: ProjectId,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let project = self.project_by_id(project_id).await?;
        self.create_section(&project, name, color).await
    }

    /// Look up a section by name within a project, ignoring case and surrounding
    /// whitespace. On a miss, suggests the closest existing name when one is
    /// close enough to be a likely typo.
    pub async fn get_section_by_name(
        &self,
        project: &Project,
        name: &str,
    ) -> Result<Section, ServiceError> {
        let name = name.trim();
        if let Some(section) = self
            .store
            .get_section_by_name(project.id, name)
            .await?
            .filter(|section| section.project_id == project.id)
        {
            return Ok(section);
        }

        let sections = self
            .store
            .get_all_sections_by_project_id(project.id)
            .await?;
        let names = sections
            .into_iter()
            .map(|section| section.name)
            .collect::<Vec<_>>();
        let suggestion = suggest_name(name, &names);
        Err(ServiceError::SectionNotFound {
            section: name.to_string(),
            project: project.name.clone(),
            suggestion,
        })
    }

    pub async fn find_section(&self, id: SectionId) -> Result<Option<Section>, ServiceError> {
        Ok(self.store.get_section_by_id(id).await?)
    }

    pub async fn find_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Option<Section>, ServiceError> {
        Ok(self.store.get_section_by_name(project_id, name).await?)
    }

    pub async fn list_sections(&self, project_id: ProjectId) -> Result<Vec<Section>, ServiceError> {
        Ok(self
            .store
            .get_all_sections_by_project_id(project_id)
            .await?)
    }

    pub async fn rename_section(
        &self,
        project: &Project,
        section_id: SectionId,
        new_name: String,
    ) -> Result<Section, ServiceError> {
        let section = self.section_in_project(project, section_id).await?;
        let name = self
            .validated_section_name(project, Some(section.id), new_name)
            .await?;
        Ok(self
            .store
            .update_section(Section { name, ..section })
            .await?)
    }

    pub async fn set_section_color(
        &self,
        project: &Project,
        section_id: SectionId,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let section = self.section_in_project(project, section_id).await?;
        Ok(self
            .store
            .update_section(Section { color, ..section })
            .await?)
    }

    pub async fn update_section_by_id(
        &self,
        section_id: SectionId,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let section = self.section_by_id(section_id).await?;
        let project = self.project_by_id(section.project_id).await?;
        let name = self
            .validated_section_name(&project, Some(section.id), name)
            .await?;
        Ok(self
            .store
            .update_section(Section {
                name,
                color,
                ..section
            })
            .await?)
    }

    /// Move a section up one position. `None` when it is already first.
    pub async fn move_section_up(
        &self,
        project: &Project,
        section_id: SectionId,
    ) -> Result<Option<Section>, ServiceError> {
        let section = self.section_in_project(project, section_id).await?;
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id)
            .await?;
        let min = sections
            .iter()
            .map(|section| section.position)
            .min()
            .unwrap_or(section.position);
        if section.position <= min {
            return Ok(None);
        }
        self.store
            .reorder_section(project.id, section.id, section.position - 1)
            .await?;
        Ok(Some(self.section_in_project(project, section.id).await?))
    }

    /// Move a section down one position. `None` when it is already last.
    pub async fn move_section_down(
        &self,
        project: &Project,
        section_id: SectionId,
    ) -> Result<Option<Section>, ServiceError> {
        let section = self.section_in_project(project, section_id).await?;
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id)
            .await?;
        let max = sections
            .iter()
            .map(|section| section.position)
            .max()
            .unwrap_or(section.position);
        if section.position >= max {
            return Ok(None);
        }
        self.store
            .reorder_section(project.id, section.id, section.position + 1)
            .await?;
        Ok(Some(self.section_in_project(project, section.id).await?))
    }

    pub async fn move_section_to(
        &self,
        project: &Project,
        section_id: SectionId,
        position: i32,
    ) -> Result<(), ServiceError> {
        self.section_in_project(project, section_id).await?;
        self.store
            .reorder_section(project.id, section_id, position)
            .await?;
        Ok(())
    }

    pub async fn move_section_by_id(
        &self,
        section_id: SectionId,
        position: i32,
    ) -> Result<(), ServiceError> {
        let section = self.section_by_id(section_id).await?;
        let project = self.project_by_id(section.project_id).await?;
        self.move_section_to(&project, section_id, position).await
    }

    pub async fn delete_section(
        &self,
        project: &Project,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.section_in_project(project, section_id).await?;
        self.store.delete_section(section.id).await?;
        Ok(section)
    }

    pub async fn delete_section_by_id(
        &self,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.section_by_id(section_id).await?;
        let project = self.project_by_id(section.project_id).await?;
        self.delete_section(&project, section_id).await
    }

    pub async fn merge_sections(
        &self,
        project: &Project,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.section_in_project(project, source_id).await?;
        let target = self.section_in_project(project, target_id).await?;
        if source.id == target.id {
            return Err(ServiceError::Storage(StorageError::Conflict(
                "cannot merge a section into itself".to_string(),
            )));
        }
        self.store.merge_sections(source.id, target.id).await?;
        Ok((source, target))
    }

    pub async fn merge_sections_by_id(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.section_by_id(source_id).await?;
        let target = self.section_by_id(target_id).await?;
        let source_project = self.project_by_id(source.project_id).await?;
        self.project_by_id(target.project_id).await?;
        self.merge_sections(&source_project, source.id, target.id)
            .await
    }

    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
        template: Option<ProjectTemplate>,
    ) -> Result<Project, ServiceError> {
        let (task_sorting_mode, show_priority) = match template {
            Some(template) => (template.task_sorting_mode, template.show_priority),
            None => (TaskSortingMode::default(), false),
        };
        let project = self
            .store
            .create_project(
                name,
                description.filter(|description| !description.is_empty()),
                task_sorting_mode,
                show_priority,
            )
            .await?;

        let Some(template) = template else {
            return Ok(project);
        };

        for section in template.sections {
            self.store
                .create_section(
                    project.id,
                    section.name.to_string(),
                    section.position,
                    section.color,
                )
                .await?;
        }

        Ok(project)
    }

    pub async fn create_project_with(
        &self,
        name: String,
        description: Option<String>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, ServiceError> {
        Ok(self
            .store
            .create_project(
                name,
                description.filter(|description| !description.is_empty()),
                task_sorting_mode,
                show_priority,
            )
            .await?)
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.store.get_all_projects().await?)
    }

    pub async fn find_project(&self, id: ProjectId) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_id(id).await?)
    }

    pub async fn find_project_by_name(&self, name: &str) -> Result<Option<Project>, ServiceError> {
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
        update: ProjectUpdate,
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

    pub async fn replace_project(&self, project: Project) -> Result<Project, ServiceError> {
        let existing = self
            .store
            .get_project_by_id(project.id)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!(
                    "project '{}' not found",
                    project.id
                )))
            })?;
        Ok(self
            .store
            .update_project(Project {
                description: project.description.filter(|d| !d.is_empty()),
                created_at: existing.created_at,
                ..project
            })
            .await?)
    }

    /// Delete a project by id, returning the deleted project. The default
    /// project cannot be deleted.
    pub async fn delete_project_by_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Project, ServiceError> {
        let project = self.project_by_id(project_id).await?;
        if project.name == "default" {
            return Err(ServiceError::Storage(StorageError::Invalid(
                "cannot delete the default project".into(),
            )));
        }
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
            SectionInput::Existing(section_id) => {
                Ok(Some(self.section_in_project(project, section_id).await?))
            }
            SectionInput::New(name) => Ok(Some(self.create_section(project, name, None).await?)),
        }
    }

    async fn validated_section_name(
        &self,
        project: &Project,
        exclude: Option<SectionId>,
        name: String,
    ) -> Result<String, ServiceError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ServiceError::SectionNameRequired);
        }
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id)
            .await?;
        if sections.iter().any(|section| {
            Some(section.id) != exclude && normalize_name(&section.name) == normalize_name(&name)
        }) {
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

    async fn next_position(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<i32, ServiceError> {
        let tasks = self.tasks_in_section(project_id, section_id).await?;
        Ok(tasks
            .iter()
            .map(|task| task.position)
            .max()
            .map_or(0, |position| position + 1))
    }

    async fn project_by_id(&self, id: ProjectId) -> Result<Project, ServiceError> {
        self.store.get_project_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("project {id} not found")))
        })
    }

    async fn task_by_id(&self, id: TaskId) -> Result<Task, ServiceError> {
        self.store.get_task_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("task {id} not found")))
        })
    }

    async fn section_by_id(&self, id: SectionId) -> Result<Section, ServiceError> {
        self.store.get_section_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("section {id} not found")))
        })
    }

    async fn task_in_project(&self, project: &Project, id: TaskId) -> Result<Task, ServiceError> {
        self.store
            .get_task_by_id(id)
            .await?
            .filter(|task| task.project_id == project.id)
            .ok_or_else(|| ServiceError::TaskNotFound {
                task_id: id,
                project: project.name.clone(),
            })
    }

    async fn section_in_project(
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
                suggestion: None,
            })
    }
}

/// Normalize a section name for case- and whitespace-insensitive comparison.
fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Levenshtein distance between two strings, measured in characters.
fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (row, left_char) in left.iter().enumerate() {
        current[0] = row + 1;
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + if left_char == right_char { 0 } else { 1 };
            current[column + 1] = (previous[column + 1] + 1)
                .min(current[column] + 1)
                .min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

/// The closest existing section name when it is within a small edit-distance
/// threshold, so a typo can be reported back with a suggestion.
fn suggest_name(typed: &str, candidates: &[String]) -> Option<String> {
    let typed = normalize_name(typed);
    let threshold = std::cmp::max(2, typed.chars().count() / 3);
    candidates
        .iter()
        .map(|candidate| {
            let distance = levenshtein_distance(&typed, &normalize_name(candidate));
            (distance, candidate)
        })
        .filter(|(distance, _)| *distance <= threshold)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::suggest_name;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn near_miss_suggests_closest_name() {
        let candidates = names(&["Backlog", "In Progress", "Review"]);
        assert_eq!(
            suggest_name("Backlogg", &candidates),
            Some("Backlog".to_string())
        );
    }

    #[test]
    fn far_off_name_has_no_suggestion() {
        let candidates = names(&["Backlog", "In Progress", "Review"]);
        assert_eq!(suggest_name("Completely Different", &candidates), None);
    }

    #[test]
    fn suggestion_normalizes_case_and_whitespace() {
        let candidates = names(&["In Progress"]);
        assert_eq!(
            suggest_name("  in progres ", &candidates),
            Some("In Progress".to_string())
        );
    }
}
