use crate::auth_store::{AuthStore, NewRefreshToken, RefreshRotation, reuse_grace};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use scry_core::error::StorageError;
use scry_core::ids::{AutomationId, Handle, NoteId, ProjectId, SectionId, TaskId};
use scry_core::models::{
    Automation, AutomationAction, AutomationTrigger, Change, ChangeKind, Color, Entity, EntityType,
    Note, Priority, Project, ProjectSnapshot, Section, Tags, Task, TaskState,
};
use scry_core::position::{Position, n_keys_between};
use scry_core::store::Store;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

// Apply batches serialize on this key so the changes log's identity sequence
// follows commit order; concurrent appends would otherwise commit out of order.
const APPLY_LOCK_KEY: i64 = 0x7363_7279;

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

// where a single store call runs: inside the caller's transaction, or on a
// connection checked out of the pool just for that call.
enum Connection<'a> {
    Transaction(&'a mut PgConnection),
    Pooled(PoolConnection<Postgres>),
}

impl Deref for Connection<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Transaction(conn) => conn,
            Self::Pooled(conn) => conn,
        }
    }
}

impl DerefMut for Connection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Transaction(conn) => conn,
            Self::Pooled(conn) => conn,
        }
    }
}

// One connection for a multi-statement atomic operation: the caller's
// connection, or a transaction this store opened and therefore must commit.
enum TransactionSource<'a> {
    Borrowed(&'a mut PgConnection),
    Owned(Transaction<'static, Postgres>),
}

impl Deref for TransactionSource<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(conn) => conn,
            Self::Owned(tx) => tx,
        }
    }
}

impl DerefMut for TransactionSource<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Borrowed(conn) => conn,
            Self::Owned(tx) => tx,
        }
    }
}

impl TransactionSource<'_> {
    /// Commit only when the store owns the transaction; a borrowed connection
    /// belongs to the caller, who commits it.
    async fn commit(self) -> Result<(), StorageError> {
        if let Self::Owned(tx) = self {
            tx.commit().await.map_err(|e| {
                StorageError::Database(format!("failed to commit transaction: {}", e))
            })?;
        }
        Ok(())
    }
}

impl PostgresStore {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .connect(database_url)
            .await
            .map_err(|e| StorageError::Database(format!("failed to connect: {}", e)))?;

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("migration failed: {}", e)))?;

        Ok(Self { pool })
    }

    async fn connection<'a>(
        &'a self,
        tx: Option<&'a mut PgConnection>,
    ) -> Result<Connection<'a>, StorageError> {
        match tx {
            Some(tx) => Ok(Connection::Transaction(tx)),
            None => {
                let conn = self.pool.acquire().await.map_err(|e| {
                    StorageError::Database(format!("failed to acquire connection: {}", e))
                })?;
                Ok(Connection::Pooled(conn))
            }
        }
    }

    /// The connection a multi-statement atomic operation runs on: the caller's
    /// connection when one is supplied, otherwise a transaction this store owns
    /// and commits.
    async fn transaction<'a>(
        &'a self,
        tx: Option<&'a mut PgConnection>,
    ) -> Result<TransactionSource<'a>, StorageError> {
        match tx {
            Some(conn) => Ok(TransactionSource::Borrowed(conn)),
            None => {
                let own = self.pool.begin().await.map_err(|e| {
                    StorageError::Database(format!("failed to begin transaction: {}", e))
                })?;
                Ok(TransactionSource::Owned(own))
            }
        }
    }
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .map(|dbe| dbe.kind() == sqlx::error::ErrorKind::UniqueViolation)
        .unwrap_or(false)
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: TaskId,
    project_id: ProjectId,
    title: String,
    description: Option<String>,
    priority: i64,
    section_id: Option<Uuid>,
    position: String,
    state: String,
    tags: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<TaskRow> for Task {
    type Error = StorageError;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        let id = row.id;
        let state = match row.state.as_str() {
            "open" => TaskState::Open,
            "closed" => TaskState::Closed,
            other => {
                return Err(StorageError::Database(format!(
                    "task {id} has invalid state {other:?}"
                )));
            }
        };

        Ok(Task {
            id: row.id,
            project_id: row.project_id,
            title: row.title,
            description: row.description,
            priority: Priority::try_from(row.priority)
                .map_err(|e| StorageError::Database(format!("invalid priority i64 value: {e}")))?,
            section_id: row.section_id,
            position: Position::parse(row.position)
                .map_err(|e| StorageError::Database(format!("invalid task position: {e}")))?,
            state,
            tags: Tags::from(row.tags.as_str()),
            created_at: row.created_at,
        })
    }
}

fn project_from_fields(
    id: ProjectId,
    name: String,
    description: Option<String>,
    task_sorting_mode: String,
    show_priority: bool,
    created_at: DateTime<Utc>,
) -> Result<Project, StorageError> {
    Ok(Project {
        id,
        name,
        description,
        task_sorting_mode: task_sorting_mode.as_str().into(),
        show_priority,
        created_at,
    })
}

fn note_from_fields(
    id: NoteId,
    task_id: TaskId,
    contents: String,
    created_at: DateTime<Utc>,
) -> Result<Note, StorageError> {
    Ok(Note {
        id,
        task_id,
        contents,
        created_at,
    })
}

fn section_from_fields(
    id: SectionId,
    project_id: ProjectId,
    name: String,
    position: String,
    color: Option<String>,
    created_at: DateTime<Utc>,
) -> Result<Section, StorageError> {
    Ok(Section {
        id,
        project_id,
        name,
        position: Position::parse(position)
            .map_err(|e| StorageError::Database(format!("invalid section position: {e}")))?,
        color: color.and_then(|c| Color::from_str(&c, false).ok()),
        created_at,
    })
}

fn automation_from_fields(
    id: AutomationId,
    project_id: ProjectId,
    trigger: serde_json::Value,
    actions: serde_json::Value,
    created_at: DateTime<Utc>,
) -> Result<Automation, StorageError> {
    let trigger = serde_json::from_value::<AutomationTrigger>(trigger)
        .map_err(|e| StorageError::Database(format!("automation {id} has invalid trigger: {e}")))?;
    let actions = serde_json::from_value::<Vec<AutomationAction>>(actions)
        .map_err(|e| StorageError::Database(format!("automation {id} has invalid actions: {e}")))?;

    Ok(Automation {
        id,
        project_id,
        trigger,
        actions,
        created_at,
    })
}

fn entity_type_from_field(value: &str) -> Result<EntityType, StorageError> {
    match value {
        "project" => Ok(EntityType::Project),
        "section" => Ok(EntityType::Section),
        "task" => Ok(EntityType::Task),
        "note" => Ok(EntityType::Note),
        "automation" => Ok(EntityType::Automation),
        other => Err(StorageError::Database(format!(
            "unknown change entity type '{other}'"
        ))),
    }
}

fn change_kind_from_field(value: &str) -> Result<ChangeKind, StorageError> {
    match value {
        "create" => Ok(ChangeKind::Create),
        "update" => Ok(ChangeKind::Update),
        "delete" => Ok(ChangeKind::Delete),
        other => Err(StorageError::Database(format!(
            "unknown change kind '{other}'"
        ))),
    }
}

#[async_trait]
impl Store for PostgresStore {
    async fn create_task(
        &self,
        task: Task,
        tx: Option<&mut PgConnection>,
    ) -> Result<Task, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                INSERT INTO tasks (id, project_id, title, description, priority, section_id, position, state, tags, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            task.id,
            task.project_id,
            &task.title,
            task.description.as_deref(),
            i64::from(task.priority),
            task.section_id,
            task.position.as_str(),
            task.state.as_str(),
            task.tags.to_string(),
            task.created_at,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(task),
                n => Err(StorageError::Database(format!(
                    "failed to add task: expected 1 row affected, got {n}"
                ))),
            },
            Err(e) => Err(StorageError::Database(format!("failed to add task: {}", e))),
        }
    }

    async fn get_task_by_id(
        &self,
        id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Task>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query_as!(
            TaskRow,
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    state,
                    tags,
                    created_at
                FROM tasks
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(row)) => Ok(Some(Task::try_from(row)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to show task: {}",
                e
            ))),
        }
    }

    async fn find_task_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<TaskId>, StorageError> {
        let mut conn = self.connection(tx).await?;
        let (low, high) = handle.range();

        // UUID ordering matches the hex-string ordering handles are built from,
        // so the padded inclusive range matches exactly the prefix.
        let ids: Vec<Uuid> = sqlx::query_scalar!(
            r#"
                SELECT id
                FROM tasks
                WHERE id BETWEEN $1 AND $2
                ORDER BY id ASC
                LIMIT $3
            "#,
            low,
            high,
            limit,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find tasks: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_task_ids(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<TaskId>, StorageError> {
        let mut conn = self.connection(tx).await?;

        let ids: Vec<Uuid> = sqlx::query_scalar!("SELECT id FROM tasks")
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("failed to list task ids: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Task>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query_as!(
            TaskRow,
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    state,
                    tags,
                    created_at
                FROM tasks
                WHERE project_id = $1
                ORDER BY position ASC, id ASC
            "#,
            project_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(Task::try_from)
                .collect::<Result<Vec<Task>, _>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list tasks: {}",
                e
            ))),
        }
    }

    async fn get_all_tasks_by_section_id(
        &self,
        section_id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Task>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query_as!(
            TaskRow,
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    state,
                    tags,
                    created_at
                FROM tasks
                WHERE section_id = $1
                ORDER BY position ASC, id ASC
            "#,
            section_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(Task::try_from)
                .collect::<Result<Vec<Task>, _>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list tasks: {}",
                e
            ))),
        }
    }

    async fn update_task(
        &self,
        task: Task,
        tx: Option<&mut PgConnection>,
    ) -> Result<Task, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                UPDATE tasks
                SET title = $1, description = $2, priority = $3, section_id = $4, position = $5, state = $6, tags = $7
                WHERE id = $8
            "#,
            &task.title,
            task.description.as_deref(),
            i64::from(task.priority),
            task.section_id,
            task.position.as_str(),
            task.state.as_str(),
            task.tags.to_string(),
            task.id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                0 => Err(StorageError::NotFound(format!(
                    "task with id '{}' not found",
                    task.id
                ))),
                _ => Ok(task),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to update task: {}",
                e
            ))),
        }
    }

    async fn delete_task(
        &self,
        id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.transaction(tx).await?;

        // notes go explicitly, like every other delete here, so the statement
        // does not depend on the foreign key's cascade
        sqlx::query!(
            r#"
                DELETE FROM notes
                WHERE task_id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete notes: {}", e)))?;

        let result = sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete task: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "task with id '{}' not found",
                id
            )));
        }

        conn.commit().await
    }

    async fn create_note(
        &self,
        note: Note,
        tx: Option<&mut PgConnection>,
    ) -> Result<Note, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                INSERT INTO notes (id, task_id, contents, created_at)
                VALUES ($1, $2, $3, $4)
            "#,
            note.id,
            note.task_id,
            &note.contents,
            note.created_at,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(note),
                n => Err(StorageError::Database(format!(
                    "failed to add note: expected 1 row affected, got {n}"
                ))),
            },
            Err(e) => Err(StorageError::Database(format!("failed to add note: {}", e))),
        }
    }

    async fn get_note_by_id(
        &self,
        id: NoteId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Note>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    task_id,
                    contents,
                    created_at
                FROM notes
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(row)) => Ok(Some(note_from_fields(
                row.id,
                row.task_id,
                row.contents,
                row.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to show note: {}",
                e
            ))),
        }
    }

    async fn get_all_notes_by_task_id(
        &self,
        task_id: TaskId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Note>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    task_id,
                    contents,
                    created_at
                FROM notes
                WHERE task_id = $1
                ORDER BY created_at ASC
            "#,
            task_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|r| note_from_fields(r.id, r.task_id, r.contents, r.created_at))
                .collect::<Result<Vec<Note>, _>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list notes: {}",
                e
            ))),
        }
    }

    async fn get_all_notes_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Note>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    n.id,
                    n.task_id,
                    n.contents,
                    n.created_at
                FROM notes n
                JOIN tasks t ON t.id = n.task_id
                WHERE t.project_id = $1
                ORDER BY n.created_at ASC
            "#,
            project_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|r| note_from_fields(r.id, r.task_id, r.contents, r.created_at))
                .collect::<Result<Vec<Note>, _>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list notes: {}",
                e
            ))),
        }
    }

    async fn update_note(
        &self,
        note: Note,
        tx: Option<&mut PgConnection>,
    ) -> Result<Note, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                UPDATE notes
                SET contents = $1
                WHERE id = $2
            "#,
            &note.contents,
            note.id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(note),
                _ => Err(StorageError::NotFound("note not found".to_string())),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to update note: {}",
                e
            ))),
        }
    }

    async fn delete_note(
        &self,
        id: NoteId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                DELETE FROM notes
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(()),
                _ => Err(StorageError::NotFound("note not found".to_string())),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to delete note: {}",
                e
            ))),
        }
    }

    async fn find_note_ids_by_handle(
        &self,
        handle: &Handle,
        limit: i64,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<NoteId>, StorageError> {
        let mut conn = self.connection(tx).await?;
        let (low, high) = handle.range();

        // See `find_task_ids_by_handle`: UUID order makes the range exact.
        let ids: Vec<Uuid> = sqlx::query_scalar!(
            r#"
                SELECT id
                FROM notes
                WHERE id BETWEEN $1 AND $2
                ORDER BY id ASC
                LIMIT $3
            "#,
            low,
            high,
            limit,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find notes: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_note_ids(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<NoteId>, StorageError> {
        let mut conn = self.connection(tx).await?;

        let ids: Vec<Uuid> = sqlx::query_scalar!("SELECT id FROM notes")
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("failed to list note ids: {}", e)))?;

        Ok(ids)
    }

    async fn create_section(
        &self,
        section: Section,
        tx: Option<&mut PgConnection>,
    ) -> Result<Section, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                INSERT INTO sections (id, project_id, name, position, color, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            section.id,
            section.project_id,
            &section.name,
            section.position.as_str(),
            section.color.map(|c| c.to_string()),
            section.created_at,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(section),
                n => Err(StorageError::Database(format!(
                    "failed to create section: expected 1 row affected, got {n}"
                ))),
            },
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "section '{}' already exists in project {}",
                section.name, section.project_id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to create section: {}",
                e
            ))),
        }
    }

    async fn get_section_by_id(
        &self,
        id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Section>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    created_at
                FROM sections
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(r)) => Ok(Some(section_from_fields(
                r.id,
                r.project_id,
                r.name,
                r.position,
                r.color,
                r.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to look up section: {}",
                e
            ))),
        }
    }

    async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Section>, StorageError> {
        let mut conn = self.connection(tx).await?;

        // Exact match: the database compares and returns names byte-for-byte.
        match sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    created_at
                FROM sections
                WHERE project_id = $1 AND name = $2
            "#,
            project_id,
            name,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(r)) => Ok(Some(section_from_fields(
                r.id,
                r.project_id,
                r.name,
                r.position,
                r.color,
                r.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to look up section: {}",
                e
            ))),
        }
    }

    async fn get_all_sections_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Section>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    created_at
                FROM sections
                WHERE project_id = $1
                ORDER BY position ASC, id ASC
            "#,
            project_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    section_from_fields(
                        r.id,
                        r.project_id,
                        r.name,
                        r.position,
                        r.color,
                        r.created_at,
                    )
                })
                .collect::<Result<Vec<Section>, StorageError>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list sections: {}",
                e
            ))),
        }
    }

    async fn update_section(
        &self,
        section: Section,
        tx: Option<&mut PgConnection>,
    ) -> Result<Section, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                UPDATE sections
                SET name = $1, color = $2, position = $3
                WHERE id = $4
            "#,
            &section.name,
            section.color.map(|c| c.to_string()),
            section.position.as_str(),
            section.id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                0 => Err(StorageError::NotFound(format!(
                    "section with id '{}' not found",
                    section.id
                ))),
                _ => Ok(section),
            },
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "section '{}' already exists in project {}",
                section.name, section.project_id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to update section: {}",
                e
            ))),
        }
    }

    async fn delete_section(
        &self,
        id: SectionId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.transaction(tx).await?;

        let section = sqlx::query!(
            r#"
                SELECT project_id
                FROM sections
                WHERE id = $1
                FOR UPDATE
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up section: {}", e)))?
        .ok_or_else(|| StorageError::NotFound(format!("section with id '{}' not found", id)))?;

        // Tasks move to the No section group, preserving their relative order and
        // landing after any tasks already there.
        let destination = sqlx::query!(
            r#"
                SELECT id, position
                FROM tasks
                WHERE project_id = $1 AND section_id IS NULL
                ORDER BY position ASC, id ASC
                FOR UPDATE
            "#,
            section.project_id,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))?;

        let source_tasks = sqlx::query!(
            r#"
                SELECT id, position
                FROM tasks
                WHERE section_id = $1
                ORDER BY position ASC, id ASC
                FOR UPDATE
            "#,
            id,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))?;

        let last_destination = match destination.last() {
            Some(task) => Some(
                Position::parse(task.position.as_str())
                    .map_err(|e| StorageError::Database(format!("invalid task position: {e}")))?,
            ),
            None => None,
        };

        let positions = n_keys_between(last_destination.as_ref(), None, source_tasks.len())
            .map_err(|e| StorageError::Invalid(format!("failed to generate position keys: {e}")))?;

        if !source_tasks.is_empty() {
            let ids: Vec<Uuid> = source_tasks.iter().map(|task| task.id).collect();
            let keys: Vec<String> = positions
                .iter()
                .map(|position| position.as_str().to_string())
                .collect();

            sqlx::query!(
                r#"
                    UPDATE tasks
                    SET section_id = NULL, position = data.position
                    FROM unnest($1::UUID[], $2::TEXT[]) AS data(id, position)
                    WHERE tasks.id = data.id
                "#,
                &ids,
                &keys,
            )
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("failed to move section tasks: {}", e)))?;
        }

        let result = sqlx::query!(
            r#"
                DELETE FROM sections
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete section: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "section with id '{}' not found",
                id
            )));
        }

        conn.commit().await
    }

    async fn create_automation(
        &self,
        automation: Automation,
        tx: Option<&mut PgConnection>,
    ) -> Result<Automation, StorageError> {
        let mut conn = self.connection(tx).await?;

        let trigger = serde_json::to_value(&automation.trigger).map_err(|e| {
            StorageError::Database(format!("failed to serialize automation trigger: {}", e))
        })?;
        let actions = serde_json::to_value(&automation.actions).map_err(|e| {
            StorageError::Database(format!("failed to serialize automation actions: {}", e))
        })?;

        match sqlx::query!(
            r#"
                INSERT INTO automations (id, project_id, trigger, actions, created_at)
                VALUES ($1, $2, $3, $4, $5)
            "#,
            automation.id,
            automation.project_id,
            trigger,
            actions,
            automation.created_at,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(automation),
                n => Err(StorageError::Database(format!(
                    "failed to create automation: expected 1 row affected, got {n}"
                ))),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to create automation: {}",
                e
            ))),
        }
    }

    async fn get_automation_by_id(
        &self,
        id: AutomationId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Automation>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    trigger,
                    actions,
                    created_at
                FROM automations
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(row)) => Ok(Some(automation_from_fields(
                row.id,
                row.project_id,
                row.trigger,
                row.actions,
                row.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to look up automation: {}",
                e
            ))),
        }
    }

    async fn get_all_automations_by_project_id(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Automation>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    trigger,
                    actions,
                    created_at
                FROM automations
                WHERE project_id = $1
                ORDER BY created_at ASC, id ASC
            "#,
            project_id,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|row| {
                    automation_from_fields(
                        row.id,
                        row.project_id,
                        row.trigger,
                        row.actions,
                        row.created_at,
                    )
                })
                .collect::<Result<Vec<Automation>, StorageError>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list automations: {}",
                e
            ))),
        }
    }

    async fn update_automation(
        &self,
        automation: Automation,
        tx: Option<&mut PgConnection>,
    ) -> Result<Automation, StorageError> {
        let mut conn = self.connection(tx).await?;

        let actions = serde_json::to_value(&automation.actions).map_err(|e| {
            StorageError::Database(format!("failed to serialize automation actions: {}", e))
        })?;

        match sqlx::query!(
            r#"
                UPDATE automations
                SET actions = $1
                WHERE id = $2
            "#,
            actions,
            automation.id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                0 => Err(StorageError::NotFound(format!(
                    "automation with id '{}' not found",
                    automation.id
                ))),
                _ => Ok(automation),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to update automation: {}",
                e
            ))),
        }
    }

    async fn delete_automation(
        &self,
        id: AutomationId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection(tx).await?;

        let result = sqlx::query!(
            r#"
                DELETE FROM automations
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete automation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "automation with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    async fn create_project(
        &self,
        project: Project,
        tx: Option<&mut PgConnection>,
    ) -> Result<Project, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                INSERT INTO projects (id, name, description, task_sorting_mode, show_priority, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            project.id,
            &project.name,
            project.description.as_deref(),
            project.task_sorting_mode.to_string(),
            project.show_priority,
            project.created_at,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(project),
                n => Err(StorageError::Database(format!(
                    "failed to create project: expected 1 row affected, got {n}"
                ))),
            },
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                project.name
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to create project: {}",
                e
            ))),
        }
    }

    async fn get_project_by_id(
        &self,
        id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Project>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(r)) => Ok(Some(project_from_fields(
                r.id,
                r.name,
                r.description,
                r.task_sorting_mode,
                r.show_priority,
                r.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to look up project: {}",
                e
            ))),
        }
    }

    async fn get_project_snapshot(
        &self,
        project_id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<ProjectSnapshot, StorageError> {
        // The four reads are independent; overlapping them hides per-query
        // latency when Postgres is reached over the network. A missing project
        // still pays for the three child reads, which is fine on the rare 404 path.
        // a caller's transaction is one connection, so those reads serialize.
        let (project, sections, tasks, notes) = match tx {
            None => tokio::try_join!(
                self.get_project_by_id(project_id, None),
                self.get_all_sections_by_project_id(project_id, None),
                self.get_all_tasks_by_project_id(project_id, None),
                self.get_all_notes_by_project_id(project_id, None),
            )?,
            Some(tx) => {
                let project = self.get_project_by_id(project_id, Some(&mut *tx)).await?;
                let sections = self
                    .get_all_sections_by_project_id(project_id, Some(&mut *tx))
                    .await?;
                let tasks = self
                    .get_all_tasks_by_project_id(project_id, Some(&mut *tx))
                    .await?;
                let notes = self
                    .get_all_notes_by_project_id(project_id, Some(&mut *tx))
                    .await?;
                (project, sections, tasks, notes)
            }
        };
        let project = project.ok_or_else(|| {
            StorageError::NotFound(format!("Project not found for id {project_id}"))
        })?;

        Ok(ProjectSnapshot {
            project,
            sections,
            tasks,
            notes,
        })
    }

    async fn get_project_by_name(
        &self,
        name: &str,
        tx: Option<&mut PgConnection>,
    ) -> Result<Option<Project>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE name = $1
            "#,
            name,
        )
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(r)) => Ok(Some(project_from_fields(
                r.id,
                r.name,
                r.description,
                r.task_sorting_mode,
                r.show_priority,
                r.created_at,
            )?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(format!(
                "failed to look up project: {}",
                e
            ))),
        }
    }

    async fn get_all_projects(
        &self,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Project>, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                ORDER BY name ASC
            "#,
        )
        .fetch_all(&mut *conn)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    project_from_fields(
                        r.id,
                        r.name,
                        r.description,
                        r.task_sorting_mode,
                        r.show_priority,
                        r.created_at,
                    )
                })
                .collect::<Result<Vec<Project>, _>>(),
            Err(e) => Err(StorageError::Database(format!(
                "failed to list projects: {}",
                e
            ))),
        }
    }

    async fn update_project(
        &self,
        project: Project,
        tx: Option<&mut PgConnection>,
    ) -> Result<Project, StorageError> {
        let mut conn = self.connection(tx).await?;

        match sqlx::query!(
            r#"
                UPDATE projects
                SET name = $1, description = $2, task_sorting_mode = $3, show_priority = $4
                WHERE id = $5
            "#,
            &project.name,
            project.description.as_deref(),
            &project.task_sorting_mode.to_string(),
            project.show_priority,
            project.id,
        )
        .execute(&mut *conn)
        .await
        {
            Ok(result) => match result.rows_affected() {
                0 => Err(StorageError::NotFound(format!(
                    "project with id '{}' not found",
                    project.id
                ))),
                _ => Ok(project),
            },
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                project.name
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to update project: {}",
                e
            ))),
        }
    }

    async fn delete_project_by_id(
        &self,
        id: ProjectId,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.transaction(tx).await?;

        // tasks go first so their notes cascade before the sections lose the
        // tasks' section references
        sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE project_id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete tasks: {}", e)))?;

        sqlx::query!(
            r#"
                DELETE FROM sections
                WHERE project_id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete sections: {}", e)))?;

        let result = sqlx::query!(
            r#"
                DELETE FROM projects
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete project: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "project with id '{}' not found",
                id
            )));
        }

        conn.commit().await
    }

    async fn is_operation_applied(
        &self,
        operation_id: Uuid,
        tx: Option<&mut PgConnection>,
    ) -> Result<bool, StorageError> {
        let mut conn = self.connection(tx).await?;

        let applied = sqlx::query_scalar!(
            r#"
                SELECT EXISTS(
                    SELECT 1 FROM applied_operations WHERE operation_id = $1
                ) AS "applied!"
            "#,
            operation_id,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to check applied operation: {}", e)))?;

        Ok(applied)
    }

    async fn record_applied_operation(
        &self,
        operation_id: Uuid,
        applied_at: DateTime<Utc>,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection(tx).await?;

        sqlx::query!(
            r#"
                INSERT INTO applied_operations (operation_id, applied_at)
                VALUES ($1, $2)
            "#,
            operation_id,
            applied_at,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to record operation: {}", e)))?;

        Ok(())
    }

    async fn record_change(
        &self,
        change: Change,
        tx: Option<&mut PgConnection>,
    ) -> Result<i64, StorageError> {
        let mut conn = self.connection(tx).await?;

        let Change {
            seq: _,
            entity_type,
            entity_id,
            kind,
            operation_id,
            client_timestamp,
            field_values,
            created_at,
        } = change;

        let seq = sqlx::query_scalar!(
            r#"
                INSERT INTO changes (entity_type, entity_id, kind, operation_id, client_timestamp, field_values, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING seq
            "#,
            entity_type.as_str(),
            entity_id,
            kind.as_str(),
            operation_id,
            client_timestamp,
            field_values,
            created_at,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to append change: {}", e)))?;

        Ok(seq)
    }

    async fn changes_after(
        &self,
        after: i64,
        tx: Option<&mut PgConnection>,
    ) -> Result<Vec<Change>, StorageError> {
        let mut conn = self.connection(tx).await?;

        let rows = sqlx::query!(
            r#"
                SELECT seq, entity_type, entity_id, kind, operation_id, client_timestamp, field_values, created_at
                FROM changes
                WHERE seq > $1
                ORDER BY seq ASC
            "#,
            after,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list changes: {}", e)))?;

        rows.into_iter()
            .map(|row| {
                Ok(Change {
                    seq: row.seq,
                    entity_type: entity_type_from_field(&row.entity_type)?,
                    entity_id: row.entity_id,
                    kind: change_kind_from_field(&row.kind)?,
                    operation_id: row.operation_id,
                    client_timestamp: row.client_timestamp,
                    field_values: row.field_values,
                    created_at: row.created_at,
                })
            })
            .collect()
    }

    async fn last_change_seq(&self, tx: Option<&mut PgConnection>) -> Result<i64, StorageError> {
        let mut conn = self.connection(tx).await?;

        let last_seq = sqlx::query_scalar!(
            r#"
                SELECT COALESCE(MAX(seq), 0) AS "max_seq!" FROM changes
            "#
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            StorageError::Database(format!("failed to read the last change seq: {}", e))
        })?;

        Ok(last_seq)
    }

    async fn is_tombstoned(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        tx: Option<&mut PgConnection>,
    ) -> Result<bool, StorageError> {
        let mut conn = self.connection(tx).await?;

        let tombstoned = sqlx::query_scalar!(
            r#"
                SELECT EXISTS(
                    SELECT 1 FROM entity_tombstones
                    WHERE entity_type = $1 AND entity_id = $2
                ) AS "tombstoned!"
            "#,
            entity_type.as_str(),
            entity_id,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to check tombstone: {}", e)))?;

        Ok(tombstoned)
    }

    async fn create_tombstone(
        &self,
        entity: Entity,
        deleted_at: DateTime<Utc>,
        tx: Option<&mut PgConnection>,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection(tx).await?;

        sqlx::query!(
            r#"
                INSERT INTO entity_tombstones (entity_type, entity_id, deleted_at)
                VALUES ($1, $2, $3)
                ON CONFLICT DO NOTHING
            "#,
            entity.entity_type().as_str(),
            entity.id(),
            deleted_at,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to tombstone entity: {}", e)))?;

        Ok(())
    }

    async fn begin_apply(&self) -> Result<Transaction<'static, Postgres>, StorageError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        // The lock is transaction scoped, so it is released when the batch
        // commits or rolls back.
        sqlx::query!("SELECT pg_advisory_xact_lock($1)", APPLY_LOCK_KEY)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("failed to acquire apply lock: {}", e)))?;

        Ok(tx)
    }
}

#[async_trait]
impl AuthStore for PostgresStore {
    async fn create_refresh_token(&self, token: NewRefreshToken) -> Result<(), StorageError> {
        let mut conn = self.connection(None).await?;

        sqlx::query!(
            r#"
                INSERT INTO refresh_tokens (token_hash, subject, origin_id, issued_at, expires_at)
                VALUES ($1, $2, $3, $4, $5)
            "#,
            token.token_hash,
            &token.subject,
            token.origin_id,
            token.issued_at,
            token.expires_at,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to create refresh token: {}", e)))?;

        Ok(())
    }

    async fn rotate_refresh_token(
        &self,
        old_hash: &[u8],
        new_hash: &[u8],
        now: DateTime<Utc>,
        new_expires_at: DateTime<Utc>,
    ) -> Result<RefreshRotation, StorageError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            StorageError::Database(format!("failed to begin refresh rotation: {}", e))
        })?;

        let row = sqlx::query!(
            r#"
                SELECT subject, origin_id, expires_at, revoked_at
                FROM refresh_tokens
                WHERE token_hash = $1
                FOR UPDATE
            "#,
            old_hash,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up refresh token: {}", e)))?;

        let Some(row) = row else {
            tx.commit()
                .await
                .map_err(|e| StorageError::Database(format!("failed to commit rotation: {}", e)))?;
            return Ok(RefreshRotation::Missing);
        };

        if let Some(revoked_at) = row.revoked_at {
            if now.signed_duration_since(revoked_at) <= reuse_grace() {
                tx.commit().await.map_err(|e| {
                    StorageError::Database(format!("failed to commit rotation: {}", e))
                })?;
                return Ok(RefreshRotation::Revoked);
            }

            // Reuse revokes the whole lineage so a stolen token cannot keep
            // rotating alongside the legitimate client.
            sqlx::query!(
                r#"
                    UPDATE refresh_tokens
                    SET revoked_at = $2
                    WHERE origin_id = $1
                "#,
                row.origin_id,
                now,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                StorageError::Database(format!("failed to revoke token lineage: {}", e))
            })?;

            tx.commit()
                .await
                .map_err(|e| StorageError::Database(format!("failed to commit rotation: {}", e)))?;
            return Ok(RefreshRotation::Reused);
        }

        if row.expires_at <= now {
            tx.commit()
                .await
                .map_err(|e| StorageError::Database(format!("failed to commit rotation: {}", e)))?;
            return Ok(RefreshRotation::Expired);
        }

        sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = $2
                WHERE token_hash = $1
            "#,
            old_hash,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!("failed to revoke old refresh token: {}", e))
        })?;

        sqlx::query!(
            r#"
                INSERT INTO refresh_tokens (token_hash, subject, origin_id, issued_at, expires_at)
                VALUES ($1, $2, $3, $4, $5)
            "#,
            new_hash,
            &row.subject,
            row.origin_id,
            now,
            new_expires_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to create refresh token: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit rotation: {}", e)))?;

        Ok(RefreshRotation::Rotated {
            subject: row.subject,
        })
    }

    async fn revoke_refresh_token(&self, token_hash: &[u8]) -> Result<(), StorageError> {
        let mut conn = self.connection(None).await?;

        sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = now()
                WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
            token_hash,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("failed to revoke refresh token: {}", e)))?;

        Ok(())
    }
}
