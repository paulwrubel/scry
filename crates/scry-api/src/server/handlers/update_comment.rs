//! Handler for updating a comment.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Comment, CommentId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a comment from the request body, keyed by the path id.
pub async fn update_comment(
    State(state): State<ScryState>,
    Path(id): Path<CommentId>,
    Json(mut comment): Json<Comment>,
) -> Response {
    comment.id = id;

    match state.service.update_comment(comment).await {
        Ok(comment) => Json(comment).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
