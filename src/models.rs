use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type TaskID = i64;
pub type ProjectID = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectID,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: i64,
    pub project_id: ProjectID,
    pub name: String,
    pub position: i32,
    pub is_completed: bool,
    pub is_entry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskID,
    pub project_id: ProjectID,
    pub title: String,
    pub description: Option<String>,
    pub state_id: i64,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
