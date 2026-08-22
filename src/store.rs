use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Project, ProjectID, Status, StatusID, Task, TaskID};

#[async_trait]
pub trait TaskStore {
    /// Add a new task to a project. The task is created in the project's entry status.
    async fn add_task(&self, title: &str, project_id: ProjectID) -> Result<Task, StorageError>;

    /// Move a task to a new status. Alias for `update_task(id, project_id, Some(status_name))`.
    async fn move_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
        status_id: StatusID,
    ) -> Result<Option<Task>, StorageError> {
        self.update_task(id, project_id, status_id).await
    }

    /// Update task properties. Currently only `status_name` is supported;
    /// future flags (title, due, priority) will be added here.
    async fn update_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
        status_id: StatusID,
    ) -> Result<Option<Task>, StorageError>;

    /// Delete a task permanently.
    async fn delete_task(&self, id: TaskID, project_id: ProjectID) -> Result<bool, StorageError>;

    /// Show full details for a single task.
    async fn show_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
    ) -> Result<Option<Task>, StorageError>;

    /// List tasks in a project, optionally filtered by status.
    /// Results are ordered by position ascending, then task ID ascending.
    async fn list_tasks(
        &self,
        project_id: ProjectID,
        status_name: Option<&str>,
    ) -> Result<Vec<Task>, StorageError>;

    /// Create a new project with default statuses (todo, done).
    /// The new project becomes the active project.
    async fn create_project(&self, name: &str) -> Result<Project, StorageError>;

    /// Delete a project and all its tasks. The "default" project cannot be deleted.
    /// If the deleted project was active, the active project resets to "default".
    async fn delete_project(&self, name: &str) -> Result<(), StorageError>;

    /// List all projects. The active project should be marked separately by the caller.
    async fn list_projects(&self) -> Result<Vec<Project>, StorageError>;

    /// Look up a project by name.
    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError>;

    /// Look up a project by ID.
    #[allow(dead_code)]
    async fn get_project_by_id(&self, id: ProjectID) -> Result<Option<Project>, StorageError>;

    /// Get the currently active project.
    async fn get_active_project(&self) -> Result<Project, StorageError>;

    /// Set the active project. Persisted across sessions.
    async fn set_active_project(&self, name: &str) -> Result<(), StorageError>;

    /// Add a new status to a project. Appended after existing statuses.
    async fn add_status(&self, project_id: ProjectID, name: &str) -> Result<Status, StorageError>;

    /// Remove a status from a project. If `force` is true, tasks in the removed
    /// status are moved to the first remaining status. The last status of a project
    /// cannot be removed.
    async fn remove_status(
        &self,
        project_id: ProjectID,
        name: &str,
        force: bool,
    ) -> Result<(), StorageError>;

    /// Rename a status within a project. All tasks referencing the old name are updated.
    async fn rename_status(
        &self,
        project_id: ProjectID,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), StorageError>;

    /// List all statuses for a project, ordered by position.
    async fn list_statuses(&self, project_id: ProjectID) -> Result<Vec<Status>, StorageError>;

    /// Look up a status in a project by name.
    async fn get_status_by_name(
        &self,
        project_id: ProjectID,
        name: &str,
    ) -> Result<Option<Status>, StorageError>;

    /// Set the color of a status. Pass `None` to clear the color.
    async fn set_status_color(
        &self,
        status_id: i64,
        color: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Rename a project. If this is the active project, the active-project
    /// config is updated to the new name.
    async fn rename_project(
        &self,
        project_id: ProjectID,
        new_name: &str,
    ) -> Result<(), StorageError>;

    /// Move a status to a new position (0-based) within its project.
    /// Other statuses are shifted to accommodate the new position.
    /// The new position is clamped to [0, number of statuses - 1].
    async fn reorder_status(
        &self,
        project_id: ProjectID,
        status_name: &str,
        new_position: i32,
    ) -> Result<(), StorageError>;
}

pub mod sqlite;
