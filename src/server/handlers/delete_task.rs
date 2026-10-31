//! Handler for deleting a task by handle.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a task by handle; responds with `204 No Content`.
pub async fn delete_task(State(state): State<ScryState>, Path(handle): Path<Handle>) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.delete_task_by_id(task.id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
