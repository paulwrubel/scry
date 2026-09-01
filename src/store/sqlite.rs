use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;

use crate::error::StorageError;
use crate::models::{
    Color, Note, NoteId, Priority, Project, ProjectId, Status, StatusId, StatusStyle, Tags, Task,
    TaskId, TaskSortingMode,
};
use crate::store::{TaskStore, TaskToCreate};

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let db_path = database_url
            .strip_prefix("sqlite://")
            .ok_or_else(|| StorageError::Invalid("expected sqlite:// database URL".into()))?;

        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::Database(format!("failed to create directory {:?}: {}", parent, e))
            })?;
        }

        let connect_options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
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

#[allow(clippy::too_many_arguments)]
fn task_from_fields(
    id: i64,
    project_id: ProjectId,
    title: String,
    description: Option<String>,
    priority: i64,
    status_id: i64,
    position: i64,
    tags: String,
    created_at: String,
) -> Result<Task, StorageError> {
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|e| StorageError::Database(format!("invalid task created_at: {}", e)))?
        .with_timezone(&Utc);

    Ok(Task {
        id,
        project_id,
        title,
        description,
        priority: Priority::try_from(priority)
            .map_err(|e| StorageError::Database(format!("invalid priority i64 value: {e}")))?,
        status_id,
        position: position as i32,
        tags: Tags::from(tags.as_str()),
        created_at,
    })
}

fn status_from_fields(
    id: i64,
    project_id: ProjectId,
    name: String,
    position: i64,
    color: Option<String>,
    style: String,
) -> Status {
    Status {
        id,
        project_id,
        name,
        position: position as i32,
        color: color.and_then(|c| Color::from_str(&c, false).ok()),
        style: style.as_str().into(),
    }
}

fn project_from_fields(
    id: i64,
    name: String,
    entry_status_id: Option<StatusId>,
    task_sorting_mode: String,
    show_priority: bool,
    created_at: String,
) -> Result<Project, StorageError> {
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|e| StorageError::Database(format!("invalid project created_at: {}", e)))?
        .with_timezone(&Utc);

    Ok(Project {
        id,
        name,
        entry_status_id,
        task_sorting_mode: task_sorting_mode.as_str().into(),
        show_priority,
        created_at,
    })
}

fn note_from_fields(
    id: i64,
    task_id: i64,
    contents: String,
    created_at: String,
) -> Result<Note, StorageError> {
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|e| StorageError::Database(format!("invalid note created_at: {}", e)))?
        .with_timezone(&Utc);

    Ok(Note {
        id,
        task_id,
        contents,
        created_at,
    })
}

#[async_trait]
impl TaskStore for SqliteStore {
    async fn create_task(&self, task: TaskToCreate) -> Result<Task, StorageError> {
        let created_at = Utc::now();

        let row = sqlx::query!(
            r#"
                INSERT INTO tasks (project_id, title, description, priority, status_id, position, tags, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING id
            "#,
            &task.project_id,
            &task.title,
            &task.description,
            i64::from(task.priority),
            &task.status_id,
            &task.position,
            task.tags.to_string(),
            created_at.to_rfc3339(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to add task: {}", e)))?;

        Ok(Task {
            id: row.id.expect("RETURNING guarantees id"),
            project_id: task.project_id,
            title: task.title,
            description: task.description,
            priority: task.priority,
            status_id: task.status_id,
            position: task.position,
            tags: task.tags,
            created_at,
        })
    }

    async fn get_task_by_id(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    status_id,
                    position,
                    tags,
                    created_at
                FROM tasks
                WHERE id = ?
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to show task: {}", e)))?;

        Ok(match row {
            Some(r) => Some(task_from_fields(
                r.id,
                r.project_id,
                r.title,
                r.description,
                r.priority,
                r.status_id,
                r.position,
                r.tags,
                r.created_at,
            )?),
            None => None,
        })
    }

    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    status_id,
                    position,
                    tags,
                    created_at
                FROM tasks
                WHERE project_id = ?
                ORDER BY position ASC, id ASC
            "#,
            project_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))?;

        rows.into_iter()
            .map(|r| {
                task_from_fields(
                    r.id,
                    r.project_id,
                    r.title,
                    r.description,
                    r.priority,
                    r.status_id,
                    r.position,
                    r.tags,
                    r.created_at,
                )
            })
            .collect::<Result<Vec<Task>, _>>()
    }

    async fn get_all_tasks_by_status_id(
        &self,
        status_id: StatusId,
    ) -> Result<Vec<Task>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    title,
                    description,
                    priority,
                    status_id,
                    position,
                    tags,
                    created_at
                FROM tasks
                WHERE status_id = ?
                ORDER BY position ASC, id ASC
            "#,
            status_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))?;

        rows.into_iter()
            .map(|r| {
                task_from_fields(
                    r.id,
                    r.project_id,
                    r.title,
                    r.description,
                    r.priority,
                    r.status_id,
                    r.position,
                    r.tags,
                    r.created_at,
                )
            })
            .collect::<Result<Vec<Task>, _>>()
    }

    async fn update_task(&self, task: Task) -> Result<Task, StorageError> {
        let result = sqlx::query!(
            r#"
                UPDATE tasks
                SET 
                    title = ?, 
                    description = ?, 
                    priority = ?, 
                    status_id = ?, 
                    position = ?, 
                    tags = ?
                WHERE id = ?
            "#,
            &task.title,
            &task.description,
            i64::from(task.priority),
            &task.status_id,
            &task.position,
            task.tags.to_string(),
            &task.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to update task: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("task not found".to_string()));
        }

        Ok(task)
    }

    async fn update_and_autoposition_task(&self, task: Task) -> Result<Task, StorageError> {
        let current = self
            .get_task_by_id(task.id)
            .await?
            .ok_or_else(|| StorageError::NotFound("task not found".to_string()))?;

        // if we're not changing the status, then the position certainly won't need to be updated, so this is a normal update
        if current.status_id == task.status_id {
            return self.update_task(task).await;
        }

        let row = sqlx::query!(
            r#"
                UPDATE tasks
                SET 
                    title = ?,
                    description = ?,
                    priority = ?,
                    status_id = ?,
                    position = (
                        SELECT COALESCE(MAX(position), -1) + 1
                        FROM tasks
                        WHERE status_id = ?
                    ), tags = ?
                WHERE id = ?
                RETURNING position
            "#,
            &task.title,
            &task.description,
            i64::from(task.priority),
            &task.status_id,
            &task.status_id,
            task.tags.to_string(),
            &task.id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound("task not found".to_string()),
            e => StorageError::Database(format!("failed to update task: {}", e)),
        })?;

        Ok(Task {
            position: row.position as i32,
            ..task
        })
    }

    async fn delete_task(&self, id: TaskId) -> Result<(), StorageError> {
        sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE id = ?
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete task: {}", e)))?;

        Ok(())
    }

    async fn create_note(&self, task_id: TaskId, contents: String) -> Result<Note, StorageError> {
        let created_at = Utc::now();

        let row = sqlx::query!(
            r#"
                INSERT INTO notes (task_id, contents, created_at)
                VALUES (?, ?, ?)
                RETURNING id
            "#,
            &task_id,
            &contents,
            created_at.to_rfc3339(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to add note: {}", e)))?;

        Ok(Note {
            id: row.id.expect("RETURNING guarantees id"),
            task_id,
            contents,
            created_at,
        })
    }

    async fn get_note_by_id(&self, id: NoteId) -> Result<Option<Note>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT
                    n.id,
                    n.task_id,
                    n.contents,
                    n.created_at
                FROM notes n
                WHERE n.id = ?
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to show note: {}", e)))?;

        Ok(match row {
            Some(r) => Some(note_from_fields(r.id, r.task_id, r.contents, r.created_at)?),
            None => None,
        })
    }

    async fn get_all_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT
                    n.id,
                    n.task_id,
                    n.contents,
                    n.created_at
                FROM notes n
                WHERE n.task_id = ?
                ORDER BY n.created_at ASC
            "#,
            task_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list notes: {}", e)))?;

        rows.into_iter()
            .map(|r| note_from_fields(r.id, r.task_id, r.contents, r.created_at))
            .collect::<Result<Vec<Note>, _>>()
    }

    async fn get_all_notes_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT
                    n.id,
                    n.task_id,
                    n.contents,
                    n.created_at
                FROM notes n
                JOIN tasks t ON t.id = n.task_id
                WHERE t.project_id = ?
                ORDER BY n.created_at ASC
            "#,
            project_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list notes: {}", e)))?;

        rows.into_iter()
            .map(|r| note_from_fields(r.id, r.task_id, r.contents, r.created_at))
            .collect::<Result<Vec<Note>, _>>()
    }

    async fn update_note(&self, note: Note) -> Result<Note, StorageError> {
        let result = sqlx::query!(
            r#"
                UPDATE notes
                SET contents = ?
                WHERE id = ?
            "#,
            &note.contents,
            &note.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to update note: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("note not found".to_string()));
        }

        Ok(note)
    }

    async fn delete_note(&self, id: NoteId) -> Result<(), StorageError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM notes
                WHERE id = ?
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete note: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("note not found".to_string()));
        }

        Ok(())
    }

    async fn create_status(
        &self,
        project_id: ProjectId,
        name: String,
        position: i32,
        color: Option<Color>,
        style: StatusStyle,
    ) -> Result<Status, StorageError> {
        let inserted = sqlx::query!(
            r#"
                INSERT INTO statuses (project_id, name, position, color, style)
                VALUES (?, ?, ?, ?, ?)
                RETURNING id
            "#,
            &project_id,
            &name,
            &position,
            &color.map(|c| c.to_string()),
            &style.to_string(),
        )
        .fetch_one(&self.pool)
        .await;

        match inserted {
            Ok(r) => Ok(Status {
                id: r.id.expect("RETURNING guarantees id"),
                project_id,
                name,
                position,
                color,
                style,
            }),
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "status '{}' already exists",
                name
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to add status: {}",
                e
            ))),
        }
    }

    async fn get_status_by_id(&self, id: StatusId) -> Result<Option<Status>, StorageError> {
        sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    style
                FROM statuses
                WHERE id = ?
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up status: {}", e)))
        .map(|r| {
            r.map(|r| status_from_fields(r.id, r.project_id, r.name, r.position, r.color, r.style))
        })
    }

    async fn get_status_by_project_id_and_status_name(
        &self,
        project_id: ProjectId,
        status_name: String,
    ) -> Result<Option<Status>, StorageError> {
        sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    style
                FROM statuses
                WHERE project_id = ? AND name = ?
            "#,
            project_id,
            status_name,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up status: {}", e)))
        .map(|r| {
            r.map(|r| status_from_fields(r.id, r.project_id, r.name, r.position, r.color, r.style))
        })
    }

    async fn get_all_statuses_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Status>, StorageError> {
        sqlx::query!(
            r#"
                SELECT
                    id,
                    project_id,
                    name,
                    position,
                    color,
                    style
                FROM statuses
                WHERE project_id = ?
                ORDER BY position ASC
            "#,
            project_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list statuses: {}", e)))
        .map(|rows| {
            rows.into_iter()
                .map(|r| {
                    status_from_fields(r.id, r.project_id, r.name, r.position, r.color, r.style)
                })
                .collect()
        })
    }

    async fn update_status(&self, status: Status) -> Result<Status, StorageError> {
        // check if this status already exists. it should!
        if self.get_status_by_id(status.id).await?.is_none() {
            return Err(StorageError::NotFound(format!(
                "status with id '{}' not found",
                status.id
            )));
        }

        // if there's a status with the name we're "updating" to...
        if let Some(other) = self
            .get_status_by_project_id_and_status_name(status.project_id, status.name.clone())
            .await?
        {
            // ...with a different id, we should NOT change it, because it's a conflict!
            if other.id != status.id {
                return Err(StorageError::Conflict(format!(
                    "status '{}' already exists",
                    status.name
                )));
            }
        }

        let result = sqlx::query!(
            r#"
                UPDATE statuses
                SET name = ?, position = ?, color = ?, style = ?
                WHERE id = ?
            "#,
            &status.name,
            &status.position,
            &status.color.map(|c| c.to_string()),
            &status.style.to_string(),
            &status.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to update status: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "status with id '{}' not found",
                status.id
            )));
        }

        Ok(status)
    }

    async fn reorder_status(
        &self,
        project_id: ProjectId,
        status_id: StatusId,
        new_position: i32,
    ) -> Result<(), StorageError> {
        let status = self.get_status_by_id(status_id).await?;

        let status = status.ok_or_else(|| {
            StorageError::NotFound(format!("status with id '{}' not found", status_id))
        })?;

        let current_pos = status.position;

        // count total statuses to clamp new_position
        let total = sqlx::query!(
            r#"
                SELECT COUNT(*) AS count
                FROM statuses
                WHERE project_id = ?
            "#,
            project_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to count statuses: {}", e)))?
        .count;

        let total_i32 = total as i32;
        let new_pos = new_position.clamp(0, total_i32 - 1);

        if new_pos == current_pos {
            return Ok(());
        }

        // shift the statuses strictly between the old and new positions toward the destination
        let (lo, hi, delta) = if new_pos > current_pos {
            (current_pos + 1, new_pos, -1)
        } else {
            (new_pos, current_pos - 1, 1)
        };

        sqlx::query!(
            r#"
                UPDATE statuses
                SET position = position + ?
                WHERE project_id = ? AND position >= ? AND position <= ?
            "#,
            delta,
            project_id,
            lo,
            hi,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to shift statuses: {}", e)))?;

        sqlx::query!(
            r#"
                UPDATE statuses
                SET position = ?
                WHERE id = ?
            "#,
            new_pos,
            status.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to update status position: {}", e)))?;

        Ok(())
    }

    async fn delete_status(&self, id: StatusId) -> Result<(), StorageError> {
        if self.get_status_by_id(id).await?.is_none() {
            return Err(StorageError::NotFound(format!(
                "status with id '{}' not found",
                id
            )));
        }

        // special case: we will NOT allow deleting a status that currently has tasks
        let row = sqlx::query!(
            r#"
                SELECT COUNT(*) AS "count: i64"
                FROM tasks
                WHERE status_id = ?
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to count tasks: {}", e)))?;

        if row.count > 0 {
            return Err(StorageError::Conflict(format!(
                "status with id '{}' has {} tasks.",
                id, row.count
            )));
        }

        sqlx::query!(
            r#"
                DELETE FROM statuses
                WHERE id = ?
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to remove status: {}", e)))?;

        Ok(())
    }

    async fn create_project(
        &self,
        name: String,
        entry_status_id: Option<StatusId>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, StorageError> {
        let created_at = Utc::now();

        let result = sqlx::query!(
            r#"
                INSERT INTO projects (name, entry_status_id, task_sorting_mode, show_priority, created_at)
                VALUES (?, ?, ?, ?, ?)
                RETURNING id
            "#,
            &name,
            entry_status_id,
            task_sorting_mode.to_string(),
            show_priority,
            created_at.to_rfc3339(),
        )
        .fetch_one(&self.pool)
        .await;

        let project = match result {
            Ok(r) => Project {
                id: r.id,
                name: name.clone(),
                entry_status_id,
                task_sorting_mode,
                show_priority,
                created_at,
            },
            Err(e) if is_unique_violation(&e) => {
                return Err(StorageError::Conflict(format!(
                    "project '{}' already exists",
                    name
                )));
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "failed to create project: {}",
                    e
                )));
            }
        };

        Ok(project)
    }

    async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT id, name, entry_status_id, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE id = ?
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up project: {}", e)))?;

        Ok(match row {
            Some(r) => Some(project_from_fields(
                r.id,
                r.name,
                r.entry_status_id,
                r.task_sorting_mode,
                r.show_priority,
                r.created_at,
            )?),
            None => None,
        })
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT id, name, entry_status_id, task_sorting_mode, show_priority, created_at
                FROM projects
                WHERE name = ?
            "#,
            name,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up project: {}", e)))?;

        Ok(match row {
            Some(r) => Some(project_from_fields(
                r.id,
                r.name,
                r.entry_status_id,
                r.task_sorting_mode,
                r.show_priority,
                r.created_at,
            )?),
            None => None,
        })
    }

    async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT id, name, entry_status_id, task_sorting_mode, show_priority, created_at
                FROM projects
                ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list projects: {}", e)))?;

        rows.into_iter()
            .map(|r| {
                project_from_fields(
                    r.id,
                    r.name,
                    r.entry_status_id,
                    r.task_sorting_mode,
                    r.show_priority,
                    r.created_at,
                )
            })
            .collect::<Result<Vec<Project>, _>>()
    }

    async fn update_project(&self, project: Project) -> Result<Project, StorageError> {
        let result = sqlx::query!(
            r#"
                UPDATE projects
                SET name = ?, entry_status_id = ?, task_sorting_mode = ?, show_priority = ?
                WHERE id = ?
            "#,
            &project.name,
            &project.entry_status_id,
            &project.task_sorting_mode.to_string(),
            &project.show_priority,
            &project.id,
        )
        .execute(&self.pool)
        .await;

        let result = match result {
            Ok(r) => r,
            Err(e) if is_unique_violation(&e) => {
                return Err(StorageError::Conflict(format!(
                    "project '{}' already exists",
                    project.name
                )));
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "failed to update project: {}",
                    e
                )));
            }
        };

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "project with id '{}' not found",
                project.id
            )));
        }

        Ok(project)
    }

    async fn delete_project(&self, name: String) -> Result<(), StorageError> {
        if name == "default" {
            return Err(StorageError::Invalid(
                "cannot delete the default project".into(),
            ));
        }

        let project = self
            .get_project_by_name(&name)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("project '{}' not found", name)))?;

        sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE project_id = ?
            "#,
            project.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete tasks: {}", e)))?;

        sqlx::query!(
            r#"
                DELETE FROM statuses
                WHERE project_id = ?
            "#,
            project.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete statuses: {}", e)))?;

        sqlx::query!(
            r#"
                DELETE FROM projects
                WHERE id = ?
            "#,
            project.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete project: {}", e)))?;

        let active = sqlx::query!(
            r#"
                SELECT value
                FROM config
                WHERE key = 'active_project'
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to read active project: {}", e)))?;

        if active.is_some_and(|r| r.value == project.id.to_string()) {
            let default = self
                .get_project_by_name("default")
                .await?
                .ok_or_else(|| StorageError::NotFound("project 'default' not found".to_string()))?;
            sqlx::query!(
                r#"
                    INSERT OR REPLACE INTO config (key, value)
                    VALUES ('active_project', ?)
                "#,
                default.id.to_string(),
            )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("failed to reset active project: {}", e))
            })?;
        }

        Ok(())
    }

    async fn get_active_project(&self) -> Result<Project, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT value
                FROM config
                WHERE key = 'active_project'
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to read active project: {}", e)))?;

        let id = if let Some(r) = row {
            r.value
                .parse::<i64>()
                .map_err(|e| StorageError::Database(format!("invalid active project id: {}", e)))?
        } else {
            let default = self
                .get_project_by_name("default")
                .await?
                .ok_or_else(|| StorageError::NotFound("project 'default' not found".to_string()))?;
            sqlx::query!(
                r#"
                    INSERT OR REPLACE INTO config (key, value)
                    VALUES ('active_project', ?)
                "#,
                default.id.to_string(),
            )
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to seed active project: {}", e)))?;
            default.id
        };

        self.get_project_by_id(id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("project with id '{}' not found", id)))
    }

    async fn set_active_project(&self, name: &str) -> Result<(), StorageError> {
        let project = self
            .get_project_by_name(name)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("project '{}' not found", name)))?;

        sqlx::query!(
            r#"
                INSERT OR REPLACE INTO config (key, value)
                VALUES ('active_project', ?)
            "#,
            project.id.to_string(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to set active project: {}", e)))?;

        Ok(())
    }
}
