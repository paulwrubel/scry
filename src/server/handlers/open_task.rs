//! Handler for opening a task by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::TaskId;

use crate::error::AppError;
use crate::server::ScryState;

/// Open a task and return it.
pub async fn open_task(State(state): State<ScryState>, Path(id): Path<TaskId>) -> Response {
    match state.service.open_task_by_id(id).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
