//! Handler for listing the notes on a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::TaskId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the notes on a task.
pub async fn get_notes_by_task(
    State(state): State<ScryState>,
    Path(task_id): Path<TaskId>,
) -> Response {
    match state.service.list_notes_by_task(task_id).await {
        Ok(notes) => Json(notes).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
