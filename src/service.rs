use crate::error::ServiceError;
use scry_core::backup::{ImportPlan, ImportReport};
use scry_core::error::StorageError;
use scry_core::models::{
    Color, Note, NoteId, Priority, Project, ProjectId, ProjectTemplate, Status, StatusId,
    StatusStyle, Tags, Task, TaskId, TaskSortingMode,
};
use scry_core::store::{ArcStore, ProjectSnapshot, TaskToCreate};

/// Partial task input, shared by create and update. `None` leaves a field
/// unchanged on update; `Some("")` on `description` clears it; `Some(Tags)` on
/// `tags` sets them (an empty `Tags` clears them). `title` is required when
/// creating a task. `status` is the id of one of the project's statuses.
#[derive(Debug, Clone, Default)]
pub struct TaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub status: Option<StatusId>,
}

/// A task plus the resolved status it now lives in, so callers can name it
/// without a second lookup.
#[derive(Debug, Clone)]
pub struct TaskChangeResult {
    pub task: Task,
    pub status: Status,
}

/// Partial project input. `None` leaves a field unchanged; `Some("")` on
/// `description` clears it.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub entry_status: EntryStatusUpdate,
    pub task_sorting_mode: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
}

/// How an update treats the project's entry status: leave it, clear it, or set
/// it to one of the project's statuses.
#[derive(Debug, Clone, Copy, Default)]
pub enum EntryStatusUpdate {
    #[default]
    Unchanged,
    Clear,
    Set(StatusId),
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

        let status = match input.status {
            Some(id) => self.status_in_project(project, id).await?,
            None => {
                let statuses = self
                    .store
                    .get_all_statuses_by_project_id(project.id)
                    .await?;
                self.default_status(&statuses, project.entry_status_id)
                    .cloned()
                    .ok_or(ServiceError::ProjectHasNoStatuses)?
            }
        };

        let position = self
            .store
            .get_all_tasks_by_status_id(status.id)
            .await?
            .iter()
            .map(|task| task.position)
            .max()
            .map_or(0, |position| position + 1);

        let task = self
            .store
            .create_task(TaskToCreate {
                project_id: project.id,
                title,
                description: input
                    .description
                    .filter(|description| !description.is_empty()),
                priority: input.priority.unwrap_or_default(),
                status_id: status.id,
                position,
                tags: input.tags.unwrap_or_default(),
            })
            .await?;

        Ok(TaskChangeResult { task, status })
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
        let source = self.task_in_project(project, id).await?;
        let status = self.status_in_project(project, source.status_id).await?;
        let task = self.store.create_task(TaskToCreate::from(&source)).await?;
        Ok(TaskChangeResult { task, status })
    }

    pub async fn list_tasks_by_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_project_id(project_id).await?)
    }

    pub async fn list_tasks_by_status(
        &self,
        status_id: StatusId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self.store.get_all_tasks_by_status_id(status_id).await?)
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
        let task = self.task_in_project(project, id).await?;
        let status_id = match patch.status {
            Some(id) => self.status_in_project(project, id).await?.id,
            None => task.status_id,
        };
        let updated = self
            .store
            .update_and_autoposition_task(Task {
                id: task.id,
                project_id: task.project_id,
                title: patch.title.unwrap_or(task.title),
                description: match patch.description {
                    Some(description) => {
                        Some(description).filter(|description| !description.is_empty())
                    }
                    None => task.description,
                },
                priority: patch.priority.unwrap_or(task.priority),
                status_id,
                position: task.position,
                tags: patch.tags.unwrap_or(task.tags),
                created_at: task.created_at,
            })
            .await?;
        Ok(updated)
    }

    pub async fn update_task_model(
        &self,
        project: &Project,
        task: Task,
    ) -> Result<Task, ServiceError> {
        let existing = self.task_in_project(project, task.id).await?;
        let validated_status = self.status_in_project(project, task.status_id).await?;
        Ok(self
            .store
            .update_and_autoposition_task(Task {
                id: existing.id,
                project_id: existing.project_id,
                title: task.title,
                description: task.description.filter(|d| !d.is_empty()),
                priority: task.priority,
                status_id: validated_status.id,
                position: existing.position,
                tags: task.tags,
                created_at: existing.created_at,
            })
            .await?)
    }

    pub async fn update_task_by_id(
        &self,
        task_id: TaskId,
        task: Task,
    ) -> Result<Task, ServiceError> {
        let existing = self.task_by_id(task_id).await?;
        let project = self.project_by_id(existing.project_id).await?;
        self.update_task_model(
            &project,
            Task {
                id: task_id,
                ..task
            },
        )
        .await
    }

    pub async fn move_task(
        &self,
        project: &Project,
        id: TaskId,
        status_id: StatusId,
    ) -> Result<TaskChangeResult, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let task = self.task_in_project(project, id).await?;
        let task = self
            .store
            .update_and_autoposition_task(Task {
                status_id: status.id,
                ..task
            })
            .await?;
        Ok(TaskChangeResult { task, status })
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

    pub async fn create_status(
        &self,
        project: &Project,
        name: String,
    ) -> Result<Status, ServiceError> {
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        let position = statuses
            .iter()
            .map(|status| status.position)
            .max()
            .map_or(0, |position| position + 1);
        self.create_status_at(project, name, position, None, StatusStyle::None)
            .await
    }

    pub async fn create_status_at(
        &self,
        project: &Project,
        name: String,
        position: i32,
        color: Option<Color>,
        style: StatusStyle,
    ) -> Result<Status, ServiceError> {
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        if statuses.iter().any(|status| status.name == name) {
            return Err(ServiceError::StatusNameTaken { status: name });
        }
        Ok(self
            .store
            .create_status(project.id, name, position, color, style)
            .await?)
    }

    pub async fn create_status_in_project(
        &self,
        project_id: ProjectId,
        name: String,
        position: i32,
        color: Option<Color>,
        style: StatusStyle,
    ) -> Result<Status, ServiceError> {
        let project = self.project_by_id(project_id).await?;
        self.create_status_at(&project, name, position, color, style)
            .await
    }

    /// Look up a status by name within a project.
    pub async fn get_status_by_name(
        &self,
        project: &Project,
        name: &str,
    ) -> Result<Status, ServiceError> {
        self.store
            .get_status_by_project_id_and_status_name(project.id, name.to_string())
            .await?
            .filter(|status| status.project_id == project.id)
            .ok_or_else(|| ServiceError::StatusNotFound {
                status: name.to_string(),
                project: project.name.clone(),
            })
    }

    pub async fn find_status(&self, id: StatusId) -> Result<Option<Status>, ServiceError> {
        Ok(self.store.get_status_by_id(id).await?)
    }

    pub async fn find_status_by_name(
        &self,
        project_id: ProjectId,
        name: String,
    ) -> Result<Option<Status>, ServiceError> {
        Ok(self
            .store
            .get_status_by_project_id_and_status_name(project_id, name)
            .await?)
    }

    pub async fn list_statuses(&self, project_id: ProjectId) -> Result<Vec<Status>, ServiceError> {
        Ok(self
            .store
            .get_all_statuses_by_project_id(project_id)
            .await?)
    }

    pub async fn rename_status(
        &self,
        project: &Project,
        status_id: StatusId,
        new_name: String,
    ) -> Result<Status, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        if statuses
            .iter()
            .any(|other| other.id != status.id && other.name == new_name)
        {
            return Err(ServiceError::StatusNameTaken { status: new_name });
        }
        Ok(self
            .store
            .update_status(Status {
                name: new_name,
                ..status
            })
            .await?)
    }

    pub async fn replace_status(
        &self,
        project: &Project,
        status: Status,
    ) -> Result<Status, ServiceError> {
        let existing = self.status_in_project(project, status.id).await?;
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        if statuses
            .iter()
            .any(|other| other.id != existing.id && other.name == status.name)
        {
            return Err(ServiceError::StatusNameTaken {
                status: status.name,
            });
        }
        Ok(self
            .store
            .update_status(Status {
                project_id: existing.project_id,
                ..status
            })
            .await?)
    }

    pub async fn replace_status_by_id(&self, status: Status) -> Result<Status, ServiceError> {
        let existing = self.status_by_id(status.id).await?;
        let project = self.project_by_id(existing.project_id).await?;
        self.replace_status(&project, status).await
    }

    /// Move a status up one position. `None` when it is already first.
    pub async fn move_status_up(
        &self,
        project: &Project,
        status_id: StatusId,
    ) -> Result<Option<Status>, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        let min = statuses
            .iter()
            .map(|status| status.position)
            .min()
            .unwrap_or(status.position);
        if status.position <= min {
            return Ok(None);
        }
        self.store
            .reorder_status(project.id, status.id, status.position - 1)
            .await?;
        Ok(Some(self.status_in_project(project, status.id).await?))
    }

    /// Move a status down one position. `None` when it is already last.
    pub async fn move_status_down(
        &self,
        project: &Project,
        status_id: StatusId,
    ) -> Result<Option<Status>, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let statuses = self
            .store
            .get_all_statuses_by_project_id(project.id)
            .await?;
        let max = statuses
            .iter()
            .map(|status| status.position)
            .max()
            .unwrap_or(status.position);
        if status.position >= max {
            return Ok(None);
        }
        self.store
            .reorder_status(project.id, status.id, status.position + 1)
            .await?;
        Ok(Some(self.status_in_project(project, status.id).await?))
    }

    pub async fn move_status_to(
        &self,
        project: &Project,
        status_id: StatusId,
        position: i32,
    ) -> Result<(), ServiceError> {
        self.status_in_project(project, status_id).await?;
        self.store
            .reorder_status(project.id, status_id, position)
            .await?;
        Ok(())
    }

    pub async fn move_status_by_id(
        &self,
        status_id: StatusId,
        position: i32,
    ) -> Result<(), ServiceError> {
        let status = self.status_by_id(status_id).await?;
        let project = self.project_by_id(status.project_id).await?;
        self.move_status_to(&project, status_id, position).await
    }

    pub async fn set_status_color(
        &self,
        project: &Project,
        status_id: StatusId,
        color: Option<Color>,
    ) -> Result<Status, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        Ok(self.store.update_status(Status { color, ..status }).await?)
    }

    pub async fn set_status_style(
        &self,
        project: &Project,
        status_id: StatusId,
        style: StatusStyle,
    ) -> Result<Status, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        Ok(self.store.update_status(Status { style, ..status }).await?)
    }

    pub async fn delete_status(
        &self,
        project: &Project,
        status_id: StatusId,
    ) -> Result<Status, ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let tasks = self.store.get_all_tasks_by_status_id(status.id).await?;
        if !tasks.is_empty() {
            return Err(ServiceError::StatusNotEmpty {
                status: status.name,
                count: tasks.len(),
            });
        }
        self.store.delete_status(status.id).await?;
        Ok(status)
    }

    pub async fn delete_status_by_id(&self, status_id: StatusId) -> Result<Status, ServiceError> {
        let status = self.status_by_id(status_id).await?;
        let project = self.project_by_id(status.project_id).await?;
        self.delete_status(&project, status_id).await
    }

    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
        template: Option<ProjectTemplate>,
    ) -> Result<Project, ServiceError> {
        let description = description.filter(|description| !description.is_empty());
        let project = self
            .store
            .create_project(name, description, None, TaskSortingMode::default(), false)
            .await?;

        let Some(template) = template else {
            return Ok(project);
        };

        let mut statuses = vec![];
        for status in template.statuses {
            statuses.push(
                self.store
                    .create_status(
                        project.id,
                        status.name.to_string(),
                        status.position,
                        status.color,
                        status.style,
                    )
                    .await?,
            );
        }

        let entry_status_id = template
            .entry_status_name
            .and_then(|name| statuses.iter().find(|s| s.name == name).map(|s| s.id));

        Ok(self
            .store
            .update_project(Project {
                entry_status_id,
                task_sorting_mode: template.task_sorting_mode,
                show_priority: template.show_priority,
                ..project
            })
            .await?)
    }

    pub async fn create_project_with(
        &self,
        name: String,
        description: Option<String>,
        entry_status_id: Option<StatusId>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, ServiceError> {
        Ok(self
            .store
            .create_project(
                name,
                description.filter(|description| !description.is_empty()),
                entry_status_id,
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
        let entry_status_id = match update.entry_status {
            EntryStatusUpdate::Unchanged => project.entry_status_id,
            EntryStatusUpdate::Clear => None,
            EntryStatusUpdate::Set(status_id) => {
                Some(self.status_in_project(project, status_id).await?.id)
            }
        };
        let description = match update.description {
            Some(value) => Some(value).filter(|value| !value.is_empty()),
            None => project.description.clone(),
        };
        Ok(self
            .store
            .update_project(Project {
                name: update.name.unwrap_or_else(|| project.name.clone()),
                description,
                entry_status_id,
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
        if let Some(entry_status_id) = project.entry_status_id {
            self.status_in_project(&existing, entry_status_id).await?;
        }
        Ok(self
            .store
            .update_project(Project {
                description: project.description.filter(|d| !d.is_empty()),
                created_at: existing.created_at,
                ..project
            })
            .await?)
    }

    /// Delete a project, returning the deleted project.
    pub async fn delete_project(&self, name: &str) -> Result<Project, ServiceError> {
        let project = self.store.get_project_by_name(name).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!(
                "project '{}' not found",
                name
            )))
        })?;
        self.store.delete_project(name.to_string()).await?;
        Ok(project)
    }

    pub async fn delete_project_by_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Project, ServiceError> {
        let project = self.project_by_id(project_id).await?;
        self.delete_project(&project.name).await
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

    pub async fn import(&self, plan: ImportPlan) -> Result<ImportReport, ServiceError> {
        Ok(self.store.import(plan).await?)
    }

    /// The status new tasks default to: the entry status if set and present,
    /// otherwise the first status by order. `None` when the project has none.
    fn default_status<'s>(
        &self,
        statuses: &'s [Status],
        entry_status_id: Option<StatusId>,
    ) -> Option<&'s Status> {
        entry_status_id
            .and_then(|id| statuses.iter().find(|status| status.id == id))
            .or_else(|| statuses.first())
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

    async fn status_by_id(&self, id: StatusId) -> Result<Status, ServiceError> {
        self.store.get_status_by_id(id).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("status {id} not found")))
        })
    }

    async fn status_in_project(
        &self,
        project: &Project,
        id: StatusId,
    ) -> Result<Status, ServiceError> {
        self.store
            .get_status_by_id(id)
            .await?
            .filter(|status| status.project_id == project.id)
            .ok_or_else(|| ServiceError::StatusNotFound {
                status: id.to_string(),
                project: project.name.clone(),
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
}
