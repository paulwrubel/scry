//! Handler for creating a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Priority, ProjectId, Tags};
use scry_core::store::TaskToCreate;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a task; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub status_id: i64,
    pub position: Option<i32>,
    pub tags: Option<Vec<String>>,
}

/// Create a task in a project and return it.
pub async fn create_task(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateTaskRequest>,
) -> Response {
    let task_to_create = TaskToCreate {
        project_id,
        title: body.title,
        description: body.description,
        priority: body.priority.unwrap_or_default(),
        status_id: body.status_id,
        position: body.position.unwrap_or(0),
        tags: body.tags.map(Tags::from).unwrap_or_default(),
    };

    match state.store.create_task(task_to_create).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
