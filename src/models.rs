use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type TaskID = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskID,
    pub title: String,
    pub description: String,
    pub is_complete: bool,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
