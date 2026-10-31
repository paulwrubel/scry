use crate::backup::{ImportPlan, ImportReport};
use crate::error::StorageError;
use crate::models::{
    Color, Lifecycle, Note, NoteId, Priority, Project, ProjectId, Section, SectionId, Tags, Task,
    TaskId, TaskSortingMode,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct TaskToCreate {
    pub project_id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub section_id: Option<SectionId>,
    pub position: i32,
    pub tags: Tags,
}

/// The content fields of a task that can be edited in place.
///
/// Section, position, and lifecycle each have their own dedicated operations and
/// are intentionally excluded from this payload.
pub struct TaskContentUpdate {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub tags: Tags,
}

/// A project together with all of its sections, tasks, and notes.
///
/// Backs the single-round-trip `GET /projects/{id}/snapshot` endpoint, and must
/// serialize both ways so the server response can be deserialized into it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub notes: Vec<Note>,
}

pub type ArcStore = Arc<dyn Store>;

#[async_trait]
pub trait Store: Send + Sync {
    /// Add a new task.
    async fn create_task(&self, task_to_create: TaskToCreate) -> Result<Task, StorageError>;

    /// Get a task by its id
    async fn get_task_by_id(&self, id: TaskId) -> Result<Option<Task>, StorageError>;

    /// List tasks in a project
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, StorageError>;

    /// List tasks in a section, including closed tasks
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_section_id(
        &self,
        section_id: SectionId,
    ) -> Result<Vec<Task>, StorageError>;

    /// Edit an existing task's content fields only.
    ///
    /// Returns the updated task, or `StorageError::NotFound` when the task does
    /// not exist.
    async fn update_task(
        &self,
        id: TaskId,
        task_content_update: TaskContentUpdate,
    ) -> Result<Task, StorageError>;

    /// Move a task to a section and position, writing both in a single statement.
    async fn move_task(
        &self,
        id: TaskId,
        section_id: Option<SectionId>,
        position: i32,
    ) -> Result<Task, StorageError>;

    /// Set a task's lifecycle, writing `closed_at` and `close_reason` together in
    /// a single statement.
    async fn set_lifecycle(&self, id: TaskId, lifecycle: Lifecycle) -> Result<Task, StorageError>;

    /// Duplicate a task as an open copy in the same section, inserting it
    /// immediately after the source by shifting later tasks in that group up by one.
    ///
    /// Performed in a single transaction.
    async fn duplicate_task(&self, id: TaskId) -> Result<Task, StorageError>;

    /// Delete a task
    async fn delete_task(&self, id: TaskId) -> Result<(), StorageError>;

    async fn create_note(&self, task_id: TaskId, contents: String) -> Result<Note, StorageError>;

    // The note read/update/delete methods below are not wired into the UI yet;
    // they are scaffolding for the future note editing and deletion features.
    #[allow(dead_code)]
    async fn get_note_by_id(&self, id: NoteId) -> Result<Option<Note>, StorageError>;

    /// List notes assigned to a Task
    ///
    /// Results are ordered by created date ascending.
    #[allow(dead_code)]
    async fn get_all_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, StorageError>;

    /// List all notes in a project
    ///
    /// Results are ordered by created date ascending.
    async fn get_all_notes_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, StorageError>;

    /// Edit an existing note
    #[allow(dead_code)]
    async fn update_note(&self, note: Note) -> Result<Note, StorageError>;

    /// Delete a note
    #[allow(dead_code)]
    async fn delete_note(&self, id: NoteId) -> Result<(), StorageError>;

    async fn create_section(
        &self,
        project_id: ProjectId,
        name: String,
        position: i32,
        color: Option<Color>,
    ) -> Result<Section, StorageError>;

    async fn get_section_by_id(&self, id: SectionId) -> Result<Option<Section>, StorageError>;

    /// Look up a section by name within a project, ignoring case and surrounding
    /// whitespace.
    async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Option<Section>, StorageError>;

    /// List all sections in a project
    ///
    /// Results are ordered by position ascending, then section ID ascending.
    async fn get_all_sections_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Section>, StorageError>;

    /// Edit an existing section, writing its name and color.
    async fn update_section(&self, section: Section) -> Result<Section, StorageError>;

    /// Move a section to a new position (0-based) within its project.
    /// Other sections are shifted to accommodate the new position.
    /// The new position is clamped to the existing section count, and sections
    /// are assigned consecutive positions in list order.
    async fn reorder_section(
        &self,
        project_id: ProjectId,
        section_id: SectionId,
        new_position: i32,
    ) -> Result<(), StorageError>;

    /// Delete a section, moving its tasks to the end of the No section group in
    /// their current relative order.
    ///
    /// Performed atomically in a single transaction.
    async fn delete_section(&self, id: SectionId) -> Result<(), StorageError>;

    /// Merge a source section into a target section, appending the source's tasks
    /// to the end of the target in their current relative order before deleting
    /// the source.
    ///
    /// Performed atomically in a single transaction.
    async fn merge_sections(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(), StorageError>;

    /// Create a new project. The description is optional freeform text.
    async fn create_project(
        &self,
        name: String,
        description: Option<String>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, StorageError>;

    async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<Project>, StorageError>;

    /// Load a project with all of its sections, tasks, and notes in one call.
    ///
    /// Returns `StorageError::NotFound` when the project does not exist.
    async fn get_project_snapshot(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectSnapshot, StorageError>;

    /// Look up a project by name.
    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError>;

    /// List all projects.
    async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError>;

    async fn update_project(&self, project: Project) -> Result<Project, StorageError>;

    async fn delete_project_by_id(&self, id: ProjectId) -> Result<(), StorageError>;

    /// Apply an import plan atomically.
    ///
    /// In `Replace` mode the existing projects listed in `plan.project_ids_to_delete`
    /// are removed (along with their sections, tasks, and notes) before the plan's
    /// projects are inserted; in `Skip` and `Fail` mode no existing data is
    /// removed. Rows are inserted with the ids and timestamps carried by the plan,
    /// so callers must resolve id collisions beforehand (see
    /// `crate::backup::plan_import`).
    async fn import(&self, plan: ImportPlan) -> Result<ImportReport, StorageError>;
}
