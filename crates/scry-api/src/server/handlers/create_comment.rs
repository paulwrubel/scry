//! Handler for creating a comment on a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a comment.
#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub contents: String,
}

/// Create a comment on a task and return it.
pub async fn create_comment(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Json(body): Json<CreateCommentRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.add_comment(task.id, body.contents).await {
        Ok(comment) => Json(comment).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
