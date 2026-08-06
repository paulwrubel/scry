use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Project, ProjectID, State, Task, TaskID};

#[async_trait]
pub trait TaskStore {
    /// Add a new task to a project. The task is created in the project's entry state.
    async fn add_task(&self, title: &str, project_id: ProjectID) -> Result<Task, StorageError>;

    /// Move a task to a new state. Alias for `update_task(id, project_id, Some(state_name))`.
    async fn move_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
        state_name: &str,
    ) -> Result<Option<Task>, StorageError> {
        self.update_task(id, project_id, Some(state_name)).await
    }

    /// Update task properties. Currently only `state_name` is supported;
    /// future flags (title, due, priority) will be added here.
    async fn update_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
        state_name: Option<&str>,
    ) -> Result<Option<Task>, StorageError>;

    /// Delete a task permanently.
    async fn delete_task(&self, id: TaskID, project_id: ProjectID) -> Result<bool, StorageError>;

    /// Show full details for a single task.
    async fn show_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
    ) -> Result<Option<Task>, StorageError>;

    /// List tasks in a project, optionally filtered by state.
    /// Results are ordered by task ID ascending.
    async fn list_tasks(
        &self,
        project_id: ProjectID,
        state_name: Option<&str>,
    ) -> Result<Vec<Task>, StorageError>;

    /// Create a new project with default states (todo, done).
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

    /// Add a new state to a project. Appended after existing states.
    async fn add_state(&self, project_id: ProjectID, name: &str) -> Result<State, StorageError>;

    /// Remove a state from a project. If `force` is true, tasks in the removed
    /// state are moved to the first remaining state. The last state of a project
    /// cannot be removed.
    async fn remove_state(
        &self,
        project_id: ProjectID,
        name: &str,
        force: bool,
    ) -> Result<(), StorageError>;

    /// Rename a state within a project. All tasks referencing the old name are updated.
    async fn rename_state(
        &self,
        project_id: ProjectID,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), StorageError>;

    /// List all states for a project, ordered by position.
    async fn list_states(&self, project_id: ProjectID) -> Result<Vec<State>, StorageError>;
}

pub mod sqlite;
