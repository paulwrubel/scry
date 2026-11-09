//! Handler for updating a note.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Note, NoteId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a note from the request body, keyed by the path id.
pub async fn update_note(
    State(state): State<ScryState>,
    Path(id): Path<NoteId>,
    Json(mut note): Json<Note>,
) -> Response {
    note.id = id;

    match state.service.update_note(note).await {
        Ok(note) => Json(note).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
