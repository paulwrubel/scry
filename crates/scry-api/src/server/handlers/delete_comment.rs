//! Handler for deleting a comment by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::CommentId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a comment by id; responds with `204 No Content` on success.
pub async fn delete_comment(State(state): State<ScryState>, Path(id): Path<CommentId>) -> Response {
    match state.service.delete_comment(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
