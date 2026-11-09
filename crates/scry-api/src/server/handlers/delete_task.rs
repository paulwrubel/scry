//! Handler for deleting a task by handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a task by handle and return the removed task.
pub async fn delete_task(State(state): State<ScryState>, Path(handle): Path<Handle>) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.delete_task_by_id(task.id).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
