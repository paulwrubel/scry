//! Handler for deleting an event by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::EventId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete an event by id and return it.
pub async fn delete_event(State(state): State<ScryState>, Path(id): Path<EventId>) -> Response {
    match state.service.delete_event(id).await {
        Ok(event) => Json(event).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
