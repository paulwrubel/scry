//! Handler for creating a note on a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::TaskId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a note.
#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub contents: String,
}

/// Create a note on a task and return it.
pub async fn create_note(
    State(state): State<ScryState>,
    Path(task_id): Path<TaskId>,
    Json(body): Json<CreateNoteRequest>,
) -> Response {
    match state.service.add_note(task_id, body.contents).await {
        Ok(note) => Json(note).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
