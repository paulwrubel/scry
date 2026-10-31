//! Handler for fetching a single task by handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a task by handle.
pub async fn get_task(State(state): State<ScryState>, Path(handle): Path<Handle>) -> Response {
    match state.service.get_task_by_handle(&handle).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
