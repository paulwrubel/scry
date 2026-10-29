//! Handler for creating a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Priority, ProjectId, Tags};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::TaskInput;

/// Request body for creating a task; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub status_id: Option<i64>,
    pub tags: Option<Vec<String>>,
}

/// Create a task in a project and return it.
pub async fn create_task(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateTaskRequest>,
) -> Response {
    let input = TaskInput {
        title: Some(body.title),
        description: body.description,
        priority: body.priority,
        tags: body.tags.map(Tags::from),
        status: body.status_id,
    };

    match state
        .service
        .create_task_in_project(project_id, input)
        .await
    {
        Ok(change) => Json(change.task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
