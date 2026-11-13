//! Wire types returned by the REST API.

use scry_core::models::{Comment, Project, Section, Task};
use serde::{Deserialize, Serialize};

/// A project with its sections and tasks, as returned by `?detailed=true`.
#[derive(Deserialize, Serialize)]
pub struct ProjectDetail {
    #[serde(flatten)]
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<TaskDetails>,
}

/// A task with optional comments and display handle.
#[derive(Deserialize, Serialize)]
pub struct TaskDetails {
    #[serde(flatten)]
    pub task: Task,
    #[serde(default)]
    pub comments: Option<Vec<Comment>>,
    #[serde(default)]
    pub handle: Option<String>,
}

/// One outcome in a batch close response.
#[derive(Deserialize, Serialize)]
pub struct BatchTaskResult {
    pub handle: String,
    pub task: Option<Task>,
    pub error: Option<BatchTaskError>,
}

/// A failed item in a batch close response.
#[derive(Deserialize, Serialize)]
pub struct BatchTaskError {
    pub kind: String,
    pub message: String,
}
