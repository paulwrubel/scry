//! Handler for deleting a status by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::StatusId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a status by id; responds with `204 No Content` on success.
pub async fn delete_status(State(state): State<ScryState>, Path(id): Path<StatusId>) -> Response {
    match state.service.delete_status_by_id(id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
