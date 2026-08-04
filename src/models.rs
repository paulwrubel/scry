use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub type TaskID = i64;
pub type ProjectID = i64;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: ProjectID,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct State {
    pub id: i64,
    pub project_id: ProjectID,
    pub name: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: TaskID,
    pub project_id: ProjectID,
    pub title: String,
    pub description: Option<String>,
    pub state_id: i64,
    // populated from JOIN with the states table, not a direct column
    pub state_name: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
