//! Handler for fetching a single comment by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::CommentId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a comment by id; responds with `null` when it does not exist.
pub async fn get_comment(State(state): State<ScryState>, Path(id): Path<CommentId>) -> Response {
    match state.service.get_comment(id).await {
        Ok(comment) => Json(comment).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
