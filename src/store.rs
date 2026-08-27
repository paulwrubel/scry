use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Color, Project, ProjectID, Status, StatusID, Style, Task, TaskID};

#[async_trait]
pub trait TaskStore {
    /// Add a new task.
    async fn create_task(
        &self,
        project_id: ProjectID,
        title: String,
        description: Option<String>,
        status_id: i64,
        position: i32,
    ) -> Result<Task, StorageError>;

    /// Get a task by its id
    async fn get_task_by_id(&self, id: TaskID) -> Result<Option<Task>, StorageError>;

    /// List tasks in a project
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectID,
    ) -> Result<Vec<Task>, StorageError>;

    /// List tasks in a status
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_status_id(
        &self,
        status_id: StatusID,
    ) -> Result<Vec<Task>, StorageError>;

    /// Edit an existing task, writing the position of the Task to the DB exactly
    async fn update_task(&self, task: Task) -> Result<Task, StorageError>;

    /// Edit an existing task, auto-updating the position to by the maximum+1 of the new target status, if changed.
    ///
    /// Callers generally should lean on calling this method instead of the generic update when in doubt.
    async fn update_and_autoposition_task(&self, task: Task) -> Result<Task, StorageError>;

    /// Delete a task
    async fn delete_task(&self, id: TaskID) -> Result<(), StorageError>;

    async fn create_status(
        &self,
        project_id: ProjectID,
        name: String,
        position: i32,
        color: Option<Color>,
        style: Style,
    ) -> Result<Status, StorageError>;

    async fn get_status_by_id(&self, id: StatusID) -> Result<Option<Status>, StorageError>;

    async fn get_status_by_project_id_and_status_name(
        &self,
        project_id: ProjectID,
        status_name: String,
    ) -> Result<Option<Status>, StorageError>;

    async fn get_all_statuses_by_project_id(
        &self,
        project_id: ProjectID,
    ) -> Result<Vec<Status>, StorageError>;

    async fn update_status(&self, status: Status) -> Result<Status, StorageError>;

    /// Move a status to a new position (0-based) within its project.
    /// Other statuses are shifted to accommodate the new position.
    /// The new position is clamped to [0, number of statuses - 1].
    async fn reorder_status(
        &self,
        project_id: ProjectID,
        status_id: StatusID,
        new_position: i32,
    ) -> Result<(), StorageError>;

    async fn delete_status(&self, id: StatusID) -> Result<(), StorageError>;

    /// Create a new project.
    async fn create_project(&self, name: String) -> Result<Project, StorageError>;

    async fn get_project_by_id(&self, id: ProjectID) -> Result<Option<Project>, StorageError>;

    /// Look up a project by name.
    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError>;

    /// List all projects.
    async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError>;

    async fn update_project(&self, project: Project) -> Result<Project, StorageError>;

    async fn delete_project(&self, name: String) -> Result<(), StorageError>;

    /// Get the currently active project.
    async fn get_active_project(&self) -> Result<Project, StorageError>;

    /// Set the active project. Persisted across sessions.
    async fn set_active_project(&self, name: &str) -> Result<(), StorageError>;
}

pub mod sqlite;
