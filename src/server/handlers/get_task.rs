//! Handler for fetching a single task by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::TaskId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a task by id; responds with `null` when it does not exist.
pub async fn get_task(State(state): State<ScryState>, Path(id): Path<TaskId>) -> Response {
    match state.service.get_task(id).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
