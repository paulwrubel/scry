use crate::error::ServiceError;
use crate::models::{
    Color, Note, Priority, Project, ProjectTemplate, Status, StatusId, StatusStyle, Tags, Task,
    TaskId, TaskSortingMode,
};
use crate::store::{TaskStore, TaskToCreate};

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

/// Frontend-agnostic, high-level operations over one project. Resolves names to
/// ids, picks defaults, computes positions, and enforces invariants before
/// delegating the raw writes to `TaskStore`.
pub struct ProjectService<'a> {
    store: &'a dyn TaskStore,
}

impl<'a> ProjectService<'a> {
    pub fn new(store: &'a dyn TaskStore) -> Self {
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

    pub async fn delete_task(&self, project: &Project, id: TaskId) -> Result<(), ServiceError> {
        self.task_in_project(project, id).await?;
        self.store.delete_task(id).await?;
        Ok(())
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
        Ok(self
            .store
            .create_status(project.id, name, position, None, StatusStyle::None)
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
        Ok(Some(status))
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
        Ok(Some(status))
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
    ) -> Result<(), ServiceError> {
        let status = self.status_in_project(project, status_id).await?;
        let tasks = self.store.get_all_tasks_by_status_id(status.id).await?;
        if !tasks.is_empty() {
            return Err(ServiceError::StatusNotEmpty {
                status: status.name,
                count: tasks.len(),
            });
        }
        self.store.delete_status(status.id).await?;
        Ok(())
    }

    pub async fn rename_project(
        &self,
        project: &Project,
        new_name: String,
    ) -> Result<Project, ServiceError> {
        Ok(self
            .store
            .update_project(Project {
                name: new_name,
                ..project.clone()
            })
            .await?)
    }

    pub async fn set_project_entry_status(
        &self,
        project: &Project,
        status_id: Option<StatusId>,
    ) -> Result<Project, ServiceError> {
        let entry_status_id = match status_id {
            Some(id) => Some(self.status_in_project(project, id).await?.id),
            None => None,
        };
        Ok(self
            .store
            .update_project(Project {
                entry_status_id,
                ..project.clone()
            })
            .await?)
    }

    pub async fn set_project_sorting_mode(
        &self,
        project: &Project,
        mode: TaskSortingMode,
    ) -> Result<Project, ServiceError> {
        Ok(self
            .store
            .update_project(Project {
                task_sorting_mode: mode,
                ..project.clone()
            })
            .await?)
    }

    pub async fn set_project_should_show_priority(
        &self,
        project: &Project,
        show: bool,
    ) -> Result<Project, ServiceError> {
        Ok(self
            .store
            .update_project(Project {
                show_priority: show,
                ..project.clone()
            })
            .await?)
    }

    /// Set the active project, persisted across sessions.
    pub async fn set_active_project(&self, name: &str) -> Result<(), ServiceError> {
        self.store.set_active_project(name).await?;
        Ok(())
    }

    pub async fn create_project(
        &self,
        name: String,
        template: Option<ProjectTemplate>,
    ) -> Result<Project, ServiceError> {
        let project = self
            .store
            .create_project(name, None, TaskSortingMode::default(), false)
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

    /// Delete a project, returning the project that is active afterwards.
    pub async fn delete_project(&self, name: &str) -> Result<Project, ServiceError> {
        self.store.delete_project(name.to_string()).await?;
        Ok(self.store.get_active_project().await?)
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
