//! Handler for updating a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Task, TaskId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a task from the request body, keyed by the path id.
pub async fn update_task(
    State(state): State<ScryState>,
    Path(id): Path<TaskId>,
    Json(mut task): Json<Task>,
) -> Response {
    task.id = id;

    match state.store.update_task(task).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
