//! Handler for deleting a task by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::TaskId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a task by id; responds with `204 No Content` on success.
pub async fn delete_task(State(state): State<ScryState>, Path(id): Path<TaskId>) -> Response {
    match state.store.delete_task(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
