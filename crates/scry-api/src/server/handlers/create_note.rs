//! Handler for creating a note on a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
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
    Path(handle): Path<Handle>,
    Json(body): Json<CreateNoteRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.add_note(task.id, body.contents).await {
        Ok(note) => Json(note).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
