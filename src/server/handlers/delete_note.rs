//! Handler for deleting a note by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::NoteId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a note by id; responds with `204 No Content` on success.
pub async fn delete_note(State(state): State<ScryState>, Path(id): Path<NoteId>) -> Response {
    match state.service.delete_note(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
