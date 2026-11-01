use crate::backup::{ImportPlan, ImportReport};
use crate::error::StorageError;
use crate::ids::{Handle, NoteId, ProjectId, SectionId, TaskId};
use crate::models::{Change, Entity, EntityType, Note, Project, ProjectSnapshot, Section, Task};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub type ArcStore = Arc<dyn Store>;

/// The persistence operations the service layer drives.
///
/// Every method takes an optional `tx`: `None` runs the statement on a pooled
/// connection, while `Some` runs it inside the caller's transaction, and the
/// caller that passed `Some` owns the commit.
#[async_trait]
pub trait Store: Send + Sync {
    /// Add a new task.
    async fn create_task(
        &self,
        task: Task,
        tx: Option<&mut PgConnection>,
    ) -> Result<Task, StorageError>;

    /// Get a task by its id
    async fn get_task_by_id(
        &self,
        id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Task>, StorageError>;

    /// Find task ids whose UUID begins with a handle prefix.
    ///
    /// The prefix is compared against the normalized dashless form, at most
    /// `limit` ids are returned in ascending UUID order, and callers pass a small
    /// limit to detect an ambiguous prefix.
    async fn find_task_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<TaskId>, StorageError>;

    /// Return every task id in the database, across all projects and including
    /// closed tasks.
    ///
    /// Display handles are unique per entity type across the whole database, so
    /// computing them requires the complete id set.
    async fn get_all_task_ids(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<TaskId>, StorageError>;

    /// List tasks in a project
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Task>, StorageError>;

    /// List tasks in a section, including closed tasks
    ///
    /// Results are ordered by position ascending, then task ID ascending.
    async fn get_all_tasks_by_section_id(
        &self,
        section_id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Task>, StorageError>;

    /// Write a task's mutable columns (everything but its id and creation time).
    ///
    /// Callers read the task, change the fields they mean to change, and write
    /// the whole row back.
    async fn update_task(
        &self,
        task: Task,
        tx: Option<&mut PgConnection>,
    ) -> Result<Task, StorageError>;

    /// Delete a task
    async fn delete_task(
        &self,
        id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Add a note to a task.
    async fn create_note(
        &self,
        note: Note,
        tx: Option<&mut PgConnection>,
    ) -> Result<Note, StorageError>;

    // The note read/update/delete methods below are not wired into the UI yet;
    // they are scaffolding for the future note editing and deletion features.
    #[allow(dead_code)]
    async fn get_note_by_id(
        &self,
        id: NoteId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Note>, StorageError>;

    /// List notes assigned to a Task
    ///
    /// Results are ordered by created date ascending.
    #[allow(dead_code)]
    async fn get_all_notes_by_task_id(
        &self,
        task_id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Note>, StorageError>;

    /// List all notes in a project
    ///
    /// Results are ordered by created date ascending.
    async fn get_all_notes_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Note>, StorageError>;

    /// Edit an existing note
    #[allow(dead_code)]
    async fn update_note(
        &self,
        note: Note,
        tx: Option<&mut PgConnection>,
    ) -> Result<Note, StorageError>;

    /// Delete a note
    #[allow(dead_code)]
    async fn delete_note(
        &self,
        id: NoteId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Find note ids whose UUID begins with a handle prefix.
    ///
    /// The prefix is compared against the normalized dashless form, at most
    /// `limit` ids are returned in ascending UUID order, and callers pass a small
    /// limit to detect an ambiguous prefix.
    async fn find_note_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<NoteId>, StorageError>;

    /// Return every note id in the database.
    ///
    /// Display handles are unique per entity type across the whole database, so
    /// computing them requires the complete id set.
    async fn get_all_note_ids(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<NoteId>, StorageError>;

    async fn create_section(
        &self,
        section: Section,
        tx: Option<&mut PgConnection>,
    ) -> Result<Section, StorageError>;

    async fn get_section_by_id(
        &self,
        id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Section>, StorageError>;

    /// Look up a section by name within a project, ignoring case and surrounding
    /// whitespace.
    async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Section>, StorageError>;

    /// List all sections in a project
    ///
    /// Results are ordered by position ascending, then section ID ascending.
    async fn get_all_sections_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Section>, StorageError>;

    /// Edit an existing section, writing its name and color.
    async fn update_section(
        &self,
        section: Section,
        tx: Option<&mut PgConnection>,
    ) -> Result<Section, StorageError>;

    /// Delete a section, moving its tasks to the end of the No section group in
    /// their current relative order.
    ///
    /// Performed atomically in a single transaction.
    async fn delete_section(
        &self,
        id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Create a new project. The description is optional freeform text.
    async fn create_project(
        &self,
        project: Project,
        tx: Option<&mut PgConnection>,
    ) -> Result<Project, StorageError>;

    async fn get_project_by_id(
        &self,
        id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Project>, StorageError>;

    /// Load a project with all of its sections, tasks, and notes in one call.
    ///
    /// Returns `StorageError::NotFound` when the project does not exist.
    async fn get_project_snapshot(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<ProjectSnapshot, StorageError>;

    /// Look up a project by name.
    async fn get_project_by_name(
        &self,
        name: &str,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Project>, StorageError>;

    /// List all projects.
    async fn get_all_projects(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Project>, StorageError>;

    async fn update_project(
        &self,
        project: Project,
        tx: Option<&mut PgConnection>,
    ) -> Result<Project, StorageError>;

    async fn delete_project_by_id(
        &self,
        id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Apply an import plan atomically.
    ///
    /// In `Replace` mode the existing projects listed in `plan.project_ids_to_delete`
    /// are removed (along with their sections, tasks, and notes) before the plan's
    /// projects are inserted; in `Skip` and `Fail` mode no existing data is
    /// removed. Rows are inserted or updated by the UUID ids and carry the
    /// timestamps held by the plan, so callers must resolve project-name
    /// collisions beforehand (see `crate::backup::plan_import`).
    async fn import(
        &self,
        plan: ImportPlan,
        tx: Option<&mut PgConnection>,
    ) -> Result<ImportReport, StorageError>;

    /// Whether the id is already in `applied_operations`, which makes the
    /// operation a retransmission to acknowledge rather than apply.
    async fn is_operation_applied(
        &self,
        operation_id: Uuid,
        tx: Option<&mut PgConnection>,
    ) -> Result<bool, StorageError>;

    /// Record an operation id as applied at the given time.
    ///
    /// Only called for an id the dedupe check found absent; a duplicate id is a
    /// bug, and the primary key aborts the batch instead of hiding it.
    async fn record_applied_operation(
        &self,
        operation_id: Uuid,
        applied_at: DateTime<Utc>,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Append a row to the changes log, returning its `seq`.
    async fn record_change(
        &self,
        change: Change,
        tx: Option<&mut PgConnection>,
    ) -> Result<i64, StorageError>;

    /// Whether a tombstone exists for the entity, meaning an earlier delete
    /// removed it and later operations against it must be rejected.
    async fn is_tombstoned(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        tx: Option<&mut PgConnection>,
    ) -> Result<bool, StorageError>;

    /// Write the permanent tombstone for one deleted entity.
    ///
    /// Succeeds without changing the timestamp if a tombstone already exists.
    async fn create_tombstone(
        &self,
        entity: Entity,
        deleted_at: DateTime<Utc>,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError>;

    /// Open the one transaction a batch of operations is applied in.
    ///
    /// Batches are serialized against each other so the changes log keeps one
    /// order; dropping the transaction rolls it back.
    async fn begin_apply(&self) -> Result<Transaction<'static, Postgres>, StorageError>;
}
