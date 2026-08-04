use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;

use crate::error::StorageError;
use crate::models::{Task, TaskID};
use crate::store::TaskStore;

fn db_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(dir).join("scry")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local").join("share").join("scry")
    }
}

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new() -> Result<Self, StorageError> {
        let dir = db_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create scry data directory {:?}: {}", dir, e))?;

        let db_path = dir.join("scry.db");

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Failed to run database migrations: {}", e))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl TaskStore for SqliteStore {
    async fn add(&self, description: &str) -> Result<Task, StorageError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"
                INSERT INTO tasks (title, is_complete, created_at)
                VALUES (?, 0, ?)
                RETURNING id
            "#,
            description,
            now,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to add task: {}", e))?;

        Ok(Task {
            id: result.id,
            title: description.to_string(),
            description: String::new(),
            is_complete: false,
            created_at: Utc::now(),
            completed_at: None,
        })
    }

    async fn complete(&self, id: TaskID) -> Result<Option<Task>, StorageError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"
                UPDATE tasks
                SET is_complete = 1, completed_at = ?
                WHERE id = ? AND is_complete = 0
            "#,
            now,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to complete task: {}", e))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let row = sqlx::query!(
            r#"
                SELECT id, title, description, is_complete, created_at, completed_at
                FROM tasks
                WHERE id = ?
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch completed task: {}", e))?;

        Ok(Some(Task {
            id: row.id,
            title: row.title,
            description: row.description.unwrap_or_default(),
            is_complete: row.is_complete != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| format!("Failed to parse created_at: {}", e))?
                .with_timezone(&Utc),
            completed_at: match row.completed_at {
                Some(s) => Some(
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map_err(|e| format!("Failed to parse completed_at: {}", e))?
                        .with_timezone(&Utc),
                ),
                None => None,
            },
        }))
    }

    async fn delete(&self, id: TaskID) -> Result<bool, StorageError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE id = ?
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete task: {}", e))?;

        Ok(result.rows_affected() > 0)
    }

    async fn list_all(&self) -> Result<Vec<Task>, StorageError> {
        let rows = sqlx::query!(
            r#"
                SELECT id, title, description, is_complete, created_at, completed_at
                FROM tasks
                ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list tasks: {}", e))?;

        let mut tasks = Vec::with_capacity(rows.len());
        for row in rows {
            tasks.push(Task {
                id: row.id,
                title: row.title,
                description: row.description.unwrap_or_default(),
                is_complete: row.is_complete != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                    .map_err(|e| format!("Failed to parse created_at: {}", e))?
                    .with_timezone(&Utc),
                completed_at: match row.completed_at {
                    Some(s) => Some(
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map_err(|e| format!("Failed to parse completed_at: {}", e))?
                            .with_timezone(&Utc),
                    ),
                    None => None,
                },
            });
        }

        Ok(tasks)
    }
}
