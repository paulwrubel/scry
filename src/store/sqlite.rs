use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;

use crate::error::StorageError;
use crate::models::{Project, ProjectID, State, Task, TaskID};
use crate::store::TaskStore;

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
            .create_if_missing(true);

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

    async fn resolve_state_id(
        &self,
        project_id: ProjectID,
        name: &str,
    ) -> Result<Option<i64>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT id
                FROM states
                WHERE project_id = ? AND name = ?
            "#,
            project_id,
            name,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up state: {}", e)))?;

        Ok(row.map(|r| r.id))
    }
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .map(|dbe| dbe.kind() == sqlx::error::ErrorKind::UniqueViolation)
        .unwrap_or(false)
}

#[async_trait]
impl TaskStore for SqliteStore {
    async fn add_task(&self, title: &str, project_id: ProjectID) -> Result<Task, StorageError> {
        let now = Utc::now().to_rfc3339();

        let state_id = sqlx::query!(
            r#"
                SELECT id
                FROM states
                WHERE project_id = ? AND is_entry = 1
                LIMIT 1
            "#,
            project_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to find entry state: {}", e)))?
        .ok_or_else(|| StorageError::NotFound("no entry state found for project".to_string()))?
        .id;

        let row = sqlx::query!(
            r#"
                INSERT INTO tasks (title, description, created_at, project_id, state_id, position)
                VALUES (?, '', ?, ?, ?, (
                    SELECT COALESCE(MAX(position), -1) + 1 
                    FROM tasks 
                    WHERE state_id = ?
                ))
                RETURNING id
            "#,
            title,
            now,
            project_id,
            state_id,
            state_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to add task: {}", e)))?;

        sqlx::query!(
            r#"
                SELECT
                    t.id AS "id: i64",
                    t.title,
                    t.description,
                    t.created_at AS "created_at: DateTime<Utc>",
                    t.project_id AS "project_id: i64",
                    t.state_id AS "state_id: i64",
                    t.position AS "position: i32"
                FROM tasks t
                WHERE t.id = ? AND t.project_id = ?
            "#,
            row.id,
            project_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to fetch new task: {}", e)))
        .map(|task| Task {
            id: task.id,
            project_id: task.project_id,
            title: task.title,
            description: task.description,
            state_id: task.state_id,
            position: task.position,
            created_at: task.created_at,
        })
    }

    async fn update_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
        state_name: Option<&str>,
    ) -> Result<Option<Task>, StorageError> {
        let name = match state_name {
            Some(s) => s,
            None => return Err(StorageError::Invalid("no fields to update".into())),
        };

        let state_id = self
            .resolve_state_id(project_id, name)
            .await?
            .ok_or_else(|| {
                StorageError::NotFound(format!("state '{}' not found in project", name))
            })?;

        let result = sqlx::query!(
            r#"
                UPDATE tasks
                SET state_id = ?, position = (
                    SELECT COALESCE(MAX(position), -1) + 1 
                    FROM tasks 
                    WHERE state_id = ?
                )
                WHERE id = ? AND project_id = ?
            "#,
            state_id,
            state_id,
            id,
            project_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to update task: {}", e)))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        sqlx::query!(
            r#"
                SELECT
                    t.id AS "id: i64",
                    t.title,
                    t.description,
                    t.created_at AS "created_at: DateTime<Utc>",
                    t.project_id AS "project_id: i64",
                    t.state_id AS "state_id: i64",
                    t.position AS "position: i32"
                FROM tasks t
                WHERE t.id = ? AND t.project_id = ?
            "#,
            id,
            project_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to fetch updated task: {}", e)))
        .map(|task| Some(Task {
            id: task.id,
            project_id: task.project_id,
            title: task.title,
            description: task.description,
            state_id: task.state_id,
            position: task.position,
            created_at: task.created_at,
        }))
    }

    async fn delete_task(&self, id: TaskID, project_id: ProjectID) -> Result<bool, StorageError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM tasks
                WHERE id = ? AND project_id = ?
            "#,
            id,
            project_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete task: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn show_task(
        &self,
        id: TaskID,
        project_id: ProjectID,
    ) -> Result<Option<Task>, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT
                    t.id AS "id: i64",
                    t.title,
                    t.description,
                    t.created_at AS "created_at: DateTime<Utc>",
                    t.project_id AS "project_id: i64",
                    t.state_id AS "state_id: i64",
                    t.position AS "position: i32"
                FROM tasks t
                WHERE t.id = ? AND t.project_id = ?
            "#,
            id,
            project_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to show task: {}", e)))?;

        Ok(row.map(|r| Task {
            id: r.id,
            project_id: r.project_id,
            title: r.title,
            description: r.description,
            state_id: r.state_id,
            position: r.position,
            created_at: r.created_at,
        }))
    }

    async fn list_tasks(
        &self,
        project_id: ProjectID,
        state_name: Option<&str>,
    ) -> Result<Vec<Task>, StorageError> {
        if let Some(name) = state_name {
            match self.resolve_state_id(project_id, name).await? {
                Some(sid) => {
                    sqlx::query!(
                        r#"
                            SELECT
                                t.id AS "id: i64",
                                t.title,
                                t.description,
                                t.created_at AS "created_at: DateTime<Utc>",
                                t.project_id AS "project_id: i64",
                                t.state_id AS "state_id: i64",
                                t.position AS "position: i32"
                            FROM tasks t
                            WHERE t.project_id = ? AND t.state_id = ?
                            ORDER BY t.position ASC, t.id ASC
                        "#,
                        project_id,
                        sid,
                    )
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))
                    .map(|rows| {
                        rows.into_iter()
                            .map(|r| Task {
                                id: r.id,
                                project_id: r.project_id,
                                title: r.title,
                                description: r.description,
                                state_id: r.state_id,
                                position: r.position,
                                created_at: r.created_at,
                            })
                            .collect()
                    })
                }
                None => return Ok(vec![]),
            }
        } else {
            sqlx::query!(
                r#"
                    SELECT
                        t.id AS "id: i64",
                        t.title,
                        t.description,
                        t.created_at AS "created_at: DateTime<Utc>",
                        t.project_id AS "project_id: i64",
                        t.state_id AS "state_id: i64",
                        t.position AS "position: i32"
                    FROM tasks t
                    WHERE t.project_id = ?
                    ORDER BY t.position ASC, t.id ASC
                "#,
                project_id,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to list tasks: {}", e)))
            .map(|rows| {
                rows.into_iter()
                    .map(|r| Task {
                        id: r.id,
                        project_id: r.project_id,
                        title: r.title,
                        description: r.description,
                        state_id: r.state_id,
                        position: r.position,
                        created_at: r.created_at,
                    })
                    .collect()
            })
        }
    }

    async fn create_project(&self, name: &str) -> Result<Project, StorageError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"
                INSERT INTO projects (name, created_at)
                VALUES (?, ?)
                RETURNING id AS "id: i64", name, created_at AS "created_at: DateTime<Utc>"
            "#,
            name,
            now,
        )
        .fetch_one(&self.pool)
        .await
        .map(|r| Project {
            id: r.id,
            name: r.name,
            created_at: r.created_at,
        });

        let project = match result {
            Ok(r) => r,
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

        sqlx::query!(
            r#"
                INSERT INTO states (project_id, name, position, is_completed, is_entry)
                VALUES (?, 'todo', 0, 0, 1), (?, 'done', 1, 1, 0)
            "#,
            project.id,
            project.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to seed states: {}", e)))?;

        sqlx::query!(
            r#"
                INSERT OR REPLACE INTO config (key, value)
                VALUES ('active_project', ?)
            "#,
            project.name,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to set active project: {}", e)))?;

        Ok(project)
    }

    async fn delete_project(&self, name: &str) -> Result<(), StorageError> {
        if name == "default" {
            return Err(StorageError::Invalid(
                "cannot delete the default project".into(),
            ));
        }

        let project = self
            .get_project_by_name(name)
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
                DELETE FROM states
                WHERE project_id = ?
            "#,
            project.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to delete states: {}", e)))?;

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

        if active.is_some_and(|r| r.value == name) {
            sqlx::query!(
                r#"
                    INSERT OR REPLACE INTO config (key, value)
                    VALUES ('active_project', 'default')
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("failed to reset active project: {}", e))
            })?;
        }

        Ok(())
    }

    async fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        sqlx::query!(
            r#"
                SELECT id AS "id: i64", name, created_at AS "created_at: DateTime<Utc>"
                FROM projects
                ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list projects: {}", e)))
        .map(|rows| {
            rows.into_iter()
                .map(|r| Project {
                    id: r.id,
                    name: r.name,
                    created_at: r.created_at,
                })
                .collect()
        })
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError> {
        sqlx::query!(
            r#"
                SELECT id AS "id: i64", name, created_at AS "created_at: DateTime<Utc>"
                FROM projects
                WHERE name = ?
            "#,
            name,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up project: {}", e)))
        .map(|opt| {
            opt.map(|r| Project {
                id: r.id,
                name: r.name,
                created_at: r.created_at,
            })
        })
    }

    async fn get_project_by_id(&self, id: ProjectID) -> Result<Option<Project>, StorageError> {
        sqlx::query!(
            r#"
                SELECT id AS "id: i64", name, created_at AS "created_at: DateTime<Utc>"
                FROM projects
                WHERE id = ?
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to look up project: {}", e)))
        .map(|opt| {
            opt.map(|r| Project {
                id: r.id,
                name: r.name,
                created_at: r.created_at,
            })
        })
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

        let name = if let Some(r) = row {
            r.value
        } else {
            sqlx::query!(
                r#"
                    INSERT OR REPLACE INTO config (key, value)
                    VALUES ('active_project', 'default')
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to seed active project: {}", e)))?;
            "default".to_string()
        };

        self.get_project_by_name(&name)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("project '{}' not found", name)))
    }

    async fn set_active_project(&self, name: &str) -> Result<(), StorageError> {
        self.get_project_by_name(name)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("project '{}' not found", name)))?;

        sqlx::query!(
            r#"
                INSERT OR REPLACE INTO config (key, value)
                VALUES ('active_project', ?)
            "#,
            name,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to set active project: {}", e)))?;

        Ok(())
    }

    async fn add_state(&self, project_id: ProjectID, name: &str) -> Result<State, StorageError> {
        let row = sqlx::query!(
            r#"
                SELECT COUNT(*) AS count
                FROM states
                WHERE project_id = ?
            "#,
            project_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to count states: {}", e)))?;

        let inserted = sqlx::query!(
            r#"
                INSERT INTO states (project_id, name, position, is_completed, is_entry)
                VALUES (?, ?, ?, 0, 0)
                RETURNING id, project_id, name, position, is_completed, is_entry
            "#,
            project_id,
            name,
            row.count,
        )
        .fetch_one(&self.pool)
        .await;

        match inserted {
            Ok(row) => Ok(State {
                id: row.id.expect("RETURNING guarantees id"),
                project_id: row.project_id,
                name: row.name,
                position: row.position as i32,
                is_completed: row.is_completed,
                is_entry: row.is_entry,
            }),
            Err(e) if is_unique_violation(&e) => Err(StorageError::Conflict(format!(
                "state '{}' already exists",
                name
            ))),
            Err(e) => Err(StorageError::Database(format!(
                "failed to add state: {}",
                e
            ))),
        }
    }

    async fn remove_state(
        &self,
        project_id: ProjectID,
        name: &str,
        force: bool,
    ) -> Result<(), StorageError> {
        let state_id = self
            .resolve_state_id(project_id, name)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("state '{}' not found", name)))?;

        let row = sqlx::query!(
            r#"
                SELECT COUNT(*) AS "count: i64"
                FROM states
                WHERE project_id = ?
            "#,
            project_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to count states: {}", e)))?;

        if row.count <= 1 {
            return Err(StorageError::Invalid(
                "cannot remove the last state of a project".into(),
            ));
        }

        let row = sqlx::query!(
            r#"
                SELECT COUNT(*) AS "count: i64"
                FROM tasks
                WHERE project_id = ? AND state_id = ?
            "#,
            project_id,
            state_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to count tasks: {}", e)))?;

        if row.count > 0 && !force {
            return Err(StorageError::Conflict(format!(
                "state '{}' has {} tasks. use force to move them",
                name, row.count
            )));
        }

        if force && row.count > 0 {
            let fallback = sqlx::query!(
                r#"
                    SELECT id
                    FROM states
                    WHERE project_id = ? AND id != ?
                    ORDER BY position ASC
                    LIMIT 1
                "#,
                project_id,
                state_id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to find fallback state: {}", e)))?;

            sqlx::query!(
                r#"
                    UPDATE tasks
                    SET state_id = ?
                    WHERE project_id = ? AND state_id = ?
                "#,
                fallback.id,
                project_id,
                state_id,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("failed to move tasks: {}", e)))?;
        }

        sqlx::query!(
            r#"
                DELETE FROM states
                WHERE project_id = ? AND id = ?
            "#,
            project_id,
            state_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to remove state: {}", e)))?;

        Ok(())
    }

    async fn rename_state(
        &self,
        project_id: ProjectID,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), StorageError> {
        let exists = self.resolve_state_id(project_id, old_name).await?.is_some();
        if !exists {
            return Err(StorageError::NotFound(format!(
                "state '{}' not found",
                old_name
            )));
        }

        if self.resolve_state_id(project_id, new_name).await?.is_some() {
            return Err(StorageError::Conflict(format!(
                "state '{}' already exists",
                new_name
            )));
        }

        sqlx::query!(
            r#"
                UPDATE states
                SET name = ?
                WHERE project_id = ? AND name = ?
            "#,
            new_name,
            project_id,
            old_name,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to rename state: {}", e)))?;

        Ok(())
    }

    async fn list_states(&self, project_id: ProjectID) -> Result<Vec<State>, StorageError> {
        sqlx::query!(
            r#"
                SELECT
                    id AS "id: i64",
                    project_id AS "project_id: i64",
                    name,
                    position AS "position: i32",
                    is_completed AS "is_completed: bool",
                    is_entry AS "is_entry: bool"
                FROM states
                WHERE project_id = ?
                ORDER BY position ASC
            "#,
            project_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("failed to list states: {}", e)))
        .map(|rows| {
            rows.into_iter()
                .map(|r| State {
                    id: r.id,
                    project_id: r.project_id,
                    name: r.name,
                    position: r.position,
                    is_completed: r.is_completed,
                    is_entry: r.is_entry,
                })
                .collect()
        })
    }
}
