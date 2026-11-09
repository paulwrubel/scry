//! Handler for fetching a single note by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::NoteId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a note by id; responds with `null` when it does not exist.
pub async fn get_note(State(state): State<ScryState>, Path(id): Path<NoteId>) -> Response {
    match state.service.get_note(id).await {
        Ok(note) => Json(note).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
