//! Wire types returned by the REST API.

use scry_core::models::{Note, Project, Section, Task};
use serde::{Deserialize, Serialize};

/// A project with its sections and tasks, as returned by `?detailed=true`.
#[derive(Deserialize, Serialize)]
pub struct ProjectDetail {
    #[serde(flatten)]
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<TaskDetails>,
}

/// A task with optional notes and display handle.
#[derive(Deserialize, Serialize)]
pub struct TaskDetails {
    #[serde(flatten)]
    pub task: Task,
    #[serde(default)]
    pub notes: Option<Vec<Note>>,
    #[serde(default)]
    pub handle: Option<String>,
}
