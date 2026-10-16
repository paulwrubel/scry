//! Handler for deleting a project by name.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a project by name; responds with `204 No Content` on success.
pub async fn delete_project(State(state): State<ScryState>, Path(name): Path<String>) -> Response {
    match state.store.delete_project(name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
