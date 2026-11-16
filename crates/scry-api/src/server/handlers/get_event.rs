//! Handler for reading an event by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::EventId;

use crate::error::AppError;
use crate::server::ScryState;

/// Return one event, or 404 when it does not exist.
pub async fn get_event(State(state): State<ScryState>, Path(id): Path<EventId>) -> Response {
    match state.service.get_event(id).await {
        Ok(Some(event)) => Json(event).into_response(),
        Ok(None) => AppError::from(crate::error::ServiceError::EventNotFound { id }).into(),
        Err(error) => AppError::from(error).into(),
    }
}
