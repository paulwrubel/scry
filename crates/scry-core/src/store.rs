use crate::backup::{ImportPlan, ImportReport};
use crate::error::StorageError;
use crate::ids::{Handle, NoteId, ProjectId, SectionId, TaskId};
use crate::models::{
    Color, Note, Priority, Project, ProjectSnapshot, Section, Tags, Task, TaskSortingMode,
};
use crate::position::Position;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TaskToCreate {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub section_id: Option<SectionId>,
    pub position: Position,
    pub tags: Tags,
}

/// The fields needed to insert a section.
pub struct SectionToCreate {
    pub id: SectionId,
    pub project_id: ProjectId,
    pub name: String,
    pub position: Position,
    pub color: Option<Color>,
}

/// The fields needed to insert a project.
pub struct ProjectToCreate {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
}

pub type ArcStore = Arc<dyn Store>;

#[async_trait]
pub trait Store: Send + Sync {
    /// Add a new task.
    async fn create_task(&self, task_to_create: TaskToCreate) -> Result<Task, StorageError>;

    /// Get a task by its id
    async fn get_task_by_id(&self, id: TaskId) -> Result<Option<Task>, StorageError>;

    /// Find task ids whose UUID begins with a handle prefix.
    ///
    /// The prefix is compared against the normalized dashless form, at most
    /// `limit` ids are returned in ascending UUID order, and callers pass a small
    /// limit to detect an ambiguous prefix.
    async fn find_task_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
    ) -> Result<Vec<TaskId>, StorageError>;

    /// Return every task id in the database, across all projects and including
    /// closed tasks.
    ///
    /// Display handles are unique per entity type across the whole database, so
    /// computing them requires the complete id set.
    async fn get_all_task_ids(&self) -> Result<Vec<TaskId>, StorageError>;

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

    /// Write a task's mutable columns (everything but its id and creation time).
    ///
    /// Callers read the task, change the fields they mean to change, and write
    /// the whole row back.
    async fn update_task(&self, task: Task) -> Result<Task, StorageError>;

    /// Delete a task
    async fn delete_task(&self, id: TaskId) -> Result<(), StorageError>;

    /// Add a note to a task. The caller supplies the note id.
    async fn create_note(
        &self,
        id: NoteId,
        task_id: TaskId,
        contents: String,
    ) -> Result<Note, StorageError>;

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

    /// Find note ids whose UUID begins with a handle prefix.
    ///
    /// The prefix is compared against the normalized dashless form, at most
    /// `limit` ids are returned in ascending UUID order, and callers pass a small
    /// limit to detect an ambiguous prefix.
    async fn find_note_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
    ) -> Result<Vec<NoteId>, StorageError>;

    /// Return every note id in the database.
    ///
    /// Display handles are unique per entity type across the whole database, so
    /// computing them requires the complete id set.
    async fn get_all_note_ids(&self) -> Result<Vec<NoteId>, StorageError>;

    async fn create_section(&self, section: SectionToCreate) -> Result<Section, StorageError>;

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

    /// Delete a section, moving its tasks to the end of the No section group in
    /// their current relative order.
    ///
    /// Performed atomically in a single transaction.
    async fn delete_section(&self, id: SectionId) -> Result<(), StorageError>;

    /// Create a new project. The description is optional freeform text.
    async fn create_project(&self, project: ProjectToCreate) -> Result<Project, StorageError>;

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
    /// removed. Rows are inserted or updated by the UUID ids and carry the
    /// timestamps held by the plan, so callers must resolve project-name
    /// collisions beforehand (see `crate::backup::plan_import`).
    async fn import(&self, plan: ImportPlan) -> Result<ImportReport, StorageError>;
}
