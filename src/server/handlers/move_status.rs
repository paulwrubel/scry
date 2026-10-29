//! Handler for moving a status to a new position within its project.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::StatusId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for moving a status within its project.
#[derive(Deserialize)]
pub struct MoveStatusRequest {
    pub position: i32,
}

/// Move a status to a new position; responds with `204 No Content` on success.
pub async fn move_status(
    State(state): State<ScryState>,
    Path(id): Path<StatusId>,
    Json(body): Json<MoveStatusRequest>,
) -> Response {
    match state.service.move_status_by_id(id, body.position).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
