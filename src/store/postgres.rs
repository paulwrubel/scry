use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use scry_core::backup::{ImportMode, ImportPlan, ImportReport};
use scry_core::error::StorageError;
use scry_core::ids::{NoteId, ProjectId, SectionId, ShortId, TaskId};
use scry_core::models::{
    CloseReason, Color, Lifecycle, Note, Priority, Project, Section, Tags, Task, TaskSortingMode,
};
use scry_core::position::{Position, key_between, n_keys_between};
use scry_core::store::{ProjectSnapshot, Store, TaskContentUpdate, TaskToCreate};
use sqlx::postgres::{PgPool, PgPoolOptions};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .connect(database_url)
            .await
            .map_err(|e| StorageError::Database(format!("failed to connect: {}", e)))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("migration failed: {}", e)))?;

        Ok(Self { pool })
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
    closed_at: Option<DateTime<Utc>>,
    close_reason: Option<String>,
    tags: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<TaskRow> for Task {
    type Error = StorageError;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        let id = row.id;
        let lifecycle = match (row.closed_at, row.close_reason.as_deref()) {
            (None, None) => Lifecycle::Open,
            (Some(closed_at), Some(close_reason)) => {
                let reason = [CloseReason::Completed, CloseReason::Cancelled]
                    .into_iter()
                    .find(|reason| reason.as_str() == close_reason)
                    .ok_or_else(|| {
                        StorageError::Database(format!(
                            "task {id} violates lifecycle invariant: closed_at {closed_at:?} with close_reason {close_reason:?}"
                        ))
                    })?;
                Lifecycle::Closed { closed_at, reason }
            }
            (closed_at, close_reason) => {
                return Err(StorageError::Database(format!(
                    "task {id} violates lifecycle invariant: closed_at {closed_at:?} with close_reason {close_reason:?}"
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
            lifecycle,
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

#[async_trait]
impl Store for PostgresStore {
    async fn create_task(&self, task: TaskToCreate) -> Result<Task, StorageError> {
        let created_at = Utc::now();

        match sqlx::query_as!(
            TaskRow,
            r#"
                INSERT INTO tasks (id, project_id, title, description, priority, section_id, position, tags, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    closed_at,
                    close_reason,
                    tags,
                    created_at
            "#,
            task.id,
            task.project_id,
            &task.title,
            task.description.as_deref(),
            i64::from(task.priority),
            task.section_id,
            task.position.as_str(),
            task.tags.to_string(),
            created_at,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Task::try_from(row),
            Err(e) => Err(StorageError::Database(format!("failed to add task: {}", e))),
        }
    }

    async fn get_task_by_id(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
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
                    closed_at,
                    close_reason,
                    tags,
                    created_at
                FROM tasks
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
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

    async fn find_task_ids_by_short_id(
        &self,
        short_id: &ShortId,
        limit: i64,
    ) -> Result<Vec<TaskId>, StorageError> {
        let (low, high) = short_id.range();

        // UUID ordering matches the hex-string ordering short ids are built from,
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find tasks: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_task_ids(&self) -> Result<Vec<TaskId>, StorageError> {
        let ids: Vec<Uuid> = sqlx::query_scalar!("SELECT id FROM tasks")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to list task ids: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, StorageError> {
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
                    closed_at,
                    close_reason,
                    tags,
                    created_at
                FROM tasks
                WHERE project_id = $1
                ORDER BY position ASC, id ASC
            "#,
            project_id,
        )
        .fetch_all(&self.pool)
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
    ) -> Result<Vec<Task>, StorageError> {
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
                    closed_at,
                    close_reason,
                    tags,
                    created_at
                FROM tasks
                WHERE section_id = $1
                ORDER BY position ASC, id ASC
            "#,
            section_id,
        )
        .fetch_all(&self.pool)
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
        id: TaskId,
        task_content_update: TaskContentUpdate,
    ) -> Result<Task, StorageError> {
        match sqlx::query_as!(
            TaskRow,
            r#"
                UPDATE tasks
                SET
                    title = $1,
                    description = $2,
                    priority = $3,
                    tags = $4
                WHERE id = $5
                RETURNING
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    closed_at,
                    close_reason,
                    tags,
                    created_at
            "#,
            &task_content_update.title,
            task_content_update.description.as_deref(),
            i64::from(task_content_update.priority),
            task_content_update.tags.to_string(),
            id,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Task::try_from(row),
            Err(sqlx::Error::RowNotFound) => Err(StorageError::NotFound(format!(
                "task with id '{}' not found",
                id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to update task: {}",
                e
            ))),
        }
    }

    async fn move_task(
        &self,
        id: TaskId,
        section_id: Option<SectionId>,
        position: Position,
    ) -> Result<Task, StorageError> {
        match sqlx::query_as!(
            TaskRow,
            r#"
                UPDATE tasks
                SET section_id = $1, position = $2
                WHERE id = $3
                RETURNING
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    closed_at,
                    close_reason,
                    tags,
                    created_at
            "#,
            section_id,
            position.as_str(),
            id,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Task::try_from(row),
            Err(sqlx::Error::RowNotFound) => Err(StorageError::NotFound(format!(
                "task with id '{}' not found",
                id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to move task: {}",
                e
            ))),
        }
    }

    async fn set_lifecycle(&self, id: TaskId, lifecycle: Lifecycle) -> Result<Task, StorageError> {
        // closed_at and close_reason are written together so the pair never disagrees
        let (closed_at, close_reason) = match lifecycle {
            Lifecycle::Open => (None, None),
            Lifecycle::Closed { closed_at, reason } => (Some(closed_at), Some(reason.as_str())),
        };

        match sqlx::query_as!(
            TaskRow,
            r#"
                UPDATE tasks
                SET closed_at = $1, close_reason = $2
                WHERE id = $3
                RETURNING
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    closed_at,
                    close_reason,
                    tags,
                    created_at
            "#,
            closed_at,
            close_reason,
            id,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Task::try_from(row),
            Err(sqlx::Error::RowNotFound) => Err(StorageError::NotFound(format!(
                "task with id '{}' not found",
                id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to set task lifecycle: {}",
                e
            ))),
        }
    }

    async fn duplicate_task(&self, id: TaskId, new_id: TaskId) -> Result<Task, StorageError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        let source = sqlx::query!(
            r#"
                SELECT
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    tags
                FROM tasks
                WHERE id = $1
                FOR UPDATE
            "#,
            id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to show task: {}", e)))?
        .ok_or_else(|| StorageError::NotFound(format!("task with id '{}' not found", id)))?;

        let source_position = Position::parse(source.position.as_str())
            .map_err(|e| StorageError::Database(format!("invalid task position: {e}")))?;

        let next = sqlx::query!(
            r#"
                SELECT position
                FROM tasks
                WHERE project_id = $1
                    AND section_id IS NOT DISTINCT FROM $2::UUID
                    AND (position, id) > ($3, $4)
                ORDER BY position ASC, id ASC
                LIMIT 1
            "#,
            source.project_id,
            source.section_id,
            source.position.as_str(),
            id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find next task: {}", e)))?;

        let next_position = match next {
            Some(row) => Some(
                Position::parse(row.position)
                    .map_err(|e| StorageError::Database(format!("invalid task position: {e}")))?,
            ),
            None => None,
        };

        let new_position = key_between(Some(&source_position), next_position.as_ref())
            .map_err(|e| StorageError::Invalid(format!("failed to generate position key: {e}")))?;

        let row = sqlx::query_as!(
            TaskRow,
            r#"
                INSERT INTO tasks (id, project_id, title, description, priority, section_id, position, closed_at, close_reason, tags, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, NULL, $8, now())
                RETURNING
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    section_id,
                    position,
                    closed_at,
                    close_reason,
                    tags,
                    created_at
            "#,
            new_id,
            source.project_id,
            &source.title,
            source.description.as_deref(),
            source.priority,
            source.section_id,
            new_position.as_str(),
            &source.tags,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to duplicate task: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit transaction: {}", e)))?;

        Task::try_from(row)
    }

    async fn delete_task(&self, id: TaskId) -> Result<(), StorageError> {
        match sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE id = $1
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        {
            Ok(result) => match result.rows_affected() {
                1 => Ok(()),
                _ => Err(StorageError::NotFound(format!(
                    "task with id '{}' not found",
                    id
                ))),
            },
            Err(e) => Err(StorageError::Database(format!(
                "failed to delete task: {}",
                e
            ))),
        }
    }

    async fn create_note(
        &self,
        id: NoteId,
        task_id: TaskId,
        contents: String,
    ) -> Result<Note, StorageError> {
        let created_at = Utc::now();

        match sqlx::query!(
            r#"
                INSERT INTO notes (id, task_id, contents, created_at)
                VALUES ($1, $2, $3, $4)
                RETURNING id, task_id, contents, created_at
            "#,
            id,
            task_id,
            &contents,
            created_at,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => note_from_fields(row.id, row.task_id, row.contents, row.created_at),
            Err(e) => Err(StorageError::Database(format!("failed to add note: {}", e))),
        }
    }

    async fn get_note_by_id(&self, id: NoteId) -> Result<Option<Note>, StorageError> {
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
        .fetch_optional(&self.pool)
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

    async fn get_all_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, StorageError> {
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
        .fetch_all(&self.pool)
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
    ) -> Result<Vec<Note>, StorageError> {
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
        .fetch_all(&self.pool)
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

    async fn update_note(&self, note: Note) -> Result<Note, StorageError> {
        match sqlx::query!(
            r#"
                UPDATE notes
                SET contents = $1
                WHERE id = $2
            "#,
            &note.contents,
            note.id,
        )
        .execute(&self.pool)
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

    async fn delete_note(&self, id: NoteId) -> Result<(), StorageError> {
        match sqlx::query!(
            r#"
                DELETE FROM notes
                WHERE id = $1
            "#,
            id,
        )
        .execute(&self.pool)
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

    async fn find_note_ids_by_short_id(
        &self,
        short_id: &ShortId,
        limit: i64,
    ) -> Result<Vec<NoteId>, StorageError> {
        let (low, high) = short_id.range();

        // See `find_task_ids_by_short_id`: UUID order makes the range exact.
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find notes: {}", e)))?;

        Ok(ids)
    }

    async fn get_all_note_ids(&self) -> Result<Vec<NoteId>, StorageError> {
        let ids: Vec<Uuid> = sqlx::query_scalar!("SELECT id FROM notes")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to list note ids: {}", e)))?;

        Ok(ids)
    }

    async fn create_section(
        &self,
        id: SectionId,
        project_id: ProjectId,
        name: String,
        position: Position,
        color: Option<Color>,
    ) -> Result<Section, StorageError> {
        match sqlx::query!(
            r#"
                INSERT INTO sections (id, project_id, name, position, color)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    created_at
            "#,
            id,
            project_id,
            &name,
            position.as_str(),
            color.map(|c| c.to_string()),
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => section_from_fields(
                row.id,
                row.project_id,
                row.name,
                row.position,
                row.color,
                row.created_at,
            ),
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "section '{}' already exists in project {}",
                name, project_id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to create section: {}",
                e
            ))),
        }
    }

    async fn get_section_by_id(&self, id: SectionId) -> Result<Option<Section>, StorageError> {
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
        .fetch_optional(&self.pool)
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
    ) -> Result<Option<Section>, StorageError> {
        // Matches the expression behind the unique index so lookups agree with
        // the constraint: surrounding whitespace and case are ignored.
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
                WHERE project_id = $1 AND lower(btrim(name)) = lower(btrim($2::TEXT))
            "#,
            project_id,
            name,
        )
        .fetch_optional(&self.pool)
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
    ) -> Result<Vec<Section>, StorageError> {
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
        .fetch_all(&self.pool)
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

    async fn update_section(&self, section: Section) -> Result<Section, StorageError> {
        match sqlx::query!(
            r#"
                UPDATE sections
                SET name = $1, color = $2
                WHERE id = $3
                RETURNING
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    created_at
            "#,
            &section.name,
            section.color.map(|c| c.to_string()),
            section.id,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => section_from_fields(
                row.id,
                row.project_id,
                row.name,
                row.position,
                row.color,
                row.created_at,
            ),
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "section '{}' already exists in project {}",
                section.name, section.project_id
            ))),
            Err(sqlx::Error::RowNotFound) => Err(StorageError::NotFound(format!(
                "section with id '{}' not found",
                section.id
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to update section: {}",
                e
            ))),
        }
    }

    async fn delete_section(&self, id: SectionId) -> Result<(), StorageError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        let section = sqlx::query!(
            r#"
                SELECT project_id
                FROM sections
                WHERE id = $1
                FOR UPDATE
            "#,
            id,
        )
        .fetch_optional(&mut *tx)
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
        .fetch_all(&mut *tx)
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
        .fetch_all(&mut *tx)
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
            .execute(&mut *tx)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete section: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "section with id '{}' not found",
                id
            )));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit transaction: {}", e)))?;

        Ok(())
    }

    async fn merge_sections(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(), StorageError> {
        if source_id == target_id {
            return Err(StorageError::Invalid(
                "cannot merge a section into itself".to_string(),
            ));
        }

        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        let source = sqlx::query!(
            r#"
                SELECT id, project_id
                FROM sections
                WHERE id = $1
                FOR UPDATE
            "#,
            source_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up section: {}", e)))?
        .ok_or_else(|| {
            StorageError::NotFound(format!("source section with id '{}' not found", source_id))
        })?;

        let target = sqlx::query!(
            r#"
                SELECT id, project_id
                FROM sections
                WHERE id = $1
                FOR UPDATE
            "#,
            target_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up section: {}", e)))?
        .ok_or_else(|| {
            StorageError::NotFound(format!("target section with id '{}' not found", target_id))
        })?;

        if source.project_id != target.project_id {
            return Err(StorageError::Invalid(format!(
                "cannot merge section '{}' from project {} into section '{}' from project {}",
                source_id, source.project_id, target_id, target.project_id
            )));
        }

        // Source tasks append after the target's existing tasks in their current
        // relative order.
        let target_tasks = sqlx::query!(
            r#"
                SELECT id, position
                FROM tasks
                WHERE section_id = $1
                ORDER BY position ASC, id ASC
                FOR UPDATE
            "#,
            target_id,
        )
        .fetch_all(&mut *tx)
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
            source_id,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))?;

        let last_target = match target_tasks.last() {
            Some(task) => Some(
                Position::parse(task.position.as_str())
                    .map_err(|e| StorageError::Database(format!("invalid task position: {e}")))?,
            ),
            None => None,
        };

        let positions = n_keys_between(last_target.as_ref(), None, source_tasks.len())
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
                    SET section_id = $1, position = data.position
                    FROM unnest($2::UUID[], $3::TEXT[]) AS data(id, position)
                    WHERE tasks.id = data.id
                "#,
                target_id,
                &ids,
                &keys,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("failed to merge section tasks: {}", e)))?;
        }

        sqlx::query!(
            r#"
                DELETE FROM sections
                WHERE id = $1
            "#,
            source_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete section: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit transaction: {}", e)))?;

        Ok(())
    }

    async fn create_project(
        &self,
        id: ProjectId,
        name: String,
        description: Option<String>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, StorageError> {
        let created_at = Utc::now();

        match sqlx::query!(
            r#"
                INSERT INTO projects (id, name, description, task_sorting_mode, show_priority, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, name, description, task_sorting_mode, show_priority, created_at
            "#,
            id,
            &name,
            description.as_deref(),
            task_sorting_mode.to_string(),
            show_priority,
            created_at,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => project_from_fields(
                row.id,
                row.name,
                row.description,
                row.task_sorting_mode,
                row.show_priority,
                row.created_at,
            ),
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                name
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to create project: {}",
                e
            ))),
        }
    }

    async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
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
    ) -> Result<ProjectSnapshot, StorageError> {
        // The four reads are independent; overlapping them hides per-query
        // latency when Postgres is reached over the network. A missing project
        // still pays for the three child reads, which is fine on the rare 404 path.
        let (project, sections, tasks, notes) = tokio::try_join!(
            self.get_project_by_id(project_id),
            self.get_all_sections_by_project_id(project_id),
            self.get_all_tasks_by_project_id(project_id),
            self.get_all_notes_by_project_id(project_id),
        )?;
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

    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError> {
        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE name = $1
            "#,
            name,
        )
        .fetch_optional(&self.pool)
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

    async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError> {
        match sqlx::query!(
            r#"
                SELECT id, name, description, task_sorting_mode, show_priority, created_at
                FROM projects
                ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
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

    async fn update_project(&self, project: Project) -> Result<Project, StorageError> {
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
        .execute(&self.pool)
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

    async fn delete_project_by_id(&self, id: ProjectId) -> Result<(), StorageError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        // tasks go first so their notes cascade before the sections lose the
        // tasks' section references
        sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE project_id = $1
            "#,
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete tasks: {}", e)))?;

        sqlx::query!(
            r#"
                DELETE FROM sections
                WHERE project_id = $1
            "#,
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete sections: {}", e)))?;

        let result = sqlx::query!(
            r#"
                DELETE FROM projects
                WHERE id = $1
            "#,
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete project: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "project with id '{}' not found",
                id
            )));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit transaction: {}", e)))?;

        Ok(())
    }

    async fn import(&self, plan: ImportPlan) -> Result<ImportReport, StorageError> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("failed to begin transaction: {}", e))
            })?;

        if plan.mode == ImportMode::Replace {
            for project_id in &plan.project_ids_to_delete {
                // notes cascade with their tasks
                sqlx::query!("DELETE FROM tasks WHERE project_id = $1", project_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("failed to delete tasks: {}", e))
                    })?;
                sqlx::query!("DELETE FROM sections WHERE project_id = $1", project_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("failed to delete sections: {}", e))
                    })?;
                sqlx::query!("DELETE FROM projects WHERE id = $1", project_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("failed to delete project: {}", e))
                    })?;
            }
        }

        let mut report = ImportReport::default();

        for backup_project in &plan.projects {
            let project = &backup_project.project;

            sqlx::query!(
                r#"
                    INSERT INTO projects (id, name, description, task_sorting_mode, show_priority, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (id) DO UPDATE SET
                        name = EXCLUDED.name,
                        description = EXCLUDED.description,
                        task_sorting_mode = EXCLUDED.task_sorting_mode,
                        show_priority = EXCLUDED.show_priority
                "#,
                project.id,
                &project.name,
                project.description.as_deref(),
                project.task_sorting_mode.to_string(),
                project.show_priority,
                project.created_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("failed to create project: {}", e)))?;
            report.projects += 1;

            for section in &backup_project.sections {
                let color = section.color.map(|c| c.to_string());
                sqlx::query!(
                    r#"
                        INSERT INTO sections (id, project_id, name, position, color, created_at)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (id) DO UPDATE SET
                            project_id = EXCLUDED.project_id,
                            name = EXCLUDED.name,
                            position = EXCLUDED.position,
                            color = EXCLUDED.color
                    "#,
                    section.id,
                    section.project_id,
                    &section.name,
                    section.position.as_str(),
                    color.as_deref(),
                    section.created_at,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("failed to add section: {}", e)))?;
                report.sections += 1;
            }

            for task in &backup_project.tasks {
                let (closed_at, close_reason) = match &task.lifecycle {
                    Lifecycle::Open => (None, None),
                    Lifecycle::Closed { closed_at, reason } => {
                        (Some(*closed_at), Some(reason.as_str()))
                    }
                };
                sqlx::query!(
                    r#"
                        INSERT INTO tasks (id, project_id, title, description, priority, section_id, position, closed_at, close_reason, tags, created_at)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                        ON CONFLICT (id) DO UPDATE SET
                            project_id = EXCLUDED.project_id,
                            title = EXCLUDED.title,
                            description = EXCLUDED.description,
                            priority = EXCLUDED.priority,
                            section_id = EXCLUDED.section_id,
                            position = EXCLUDED.position,
                            closed_at = EXCLUDED.closed_at,
                            close_reason = EXCLUDED.close_reason,
                            tags = EXCLUDED.tags
                    "#,
                    task.id,
                    task.project_id,
                    &task.title,
                    task.description.as_deref(),
                    i64::from(task.priority),
                    task.section_id,
                    task.position.as_str(),
                    closed_at,
                    close_reason,
                    task.tags.to_string(),
                    task.created_at,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("failed to add task: {}", e)))?;
                report.tasks += 1;
            }

            for note in &backup_project.notes {
                sqlx::query!(
                    r#"
                        INSERT INTO notes (id, task_id, contents, created_at)
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT (id) DO UPDATE SET
                            task_id = EXCLUDED.task_id,
                            contents = EXCLUDED.contents
                    "#,
                    note.id,
                    note.task_id,
                    &note.contents,
                    note.created_at,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("failed to add note: {}", e)))?;
                report.notes += 1;
            }
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("failed to commit transaction: {}", e)))?;

        Ok(report)
    }
}
