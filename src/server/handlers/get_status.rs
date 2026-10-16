//! Handler for fetching a single status by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::StatusId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a status by id; responds with `null` when it does not exist.
pub async fn get_status(State(state): State<ScryState>, Path(id): Path<StatusId>) -> Response {
    match state.store.get_status_by_id(id).await {
        Ok(status) => Json(status).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
