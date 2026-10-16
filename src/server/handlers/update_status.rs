//! Handler for updating a status.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Status, StatusId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a status; the id is taken from the path, not the body.
pub async fn update_status(
    State(state): State<ScryState>,
    Path(id): Path<StatusId>,
    Json(mut status): Json<Status>,
) -> Response {
    status.id = id;
    match state.store.update_status(status).await {
        Ok(status) => Json(status).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
