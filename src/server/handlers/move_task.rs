//! Handler for moving a task, auto-positioning it within its status.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Task, TaskId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a task and auto-position it within its (possibly new) status.
///
/// The service validates the task and target status against the project, then
/// auto-positions it; changing `status_id` moves the task.
pub async fn move_task(
    State(state): State<ScryState>,
    Path(id): Path<TaskId>,
    Json(mut task): Json<Task>,
) -> Response {
    task.id = id;

    match state.service.update_task_by_id(id, task).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
