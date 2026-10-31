//! Handler for moving a section to a new position within its project.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for moving a section.
#[derive(Deserialize)]
pub struct MoveSectionRequest {
    pub position: i32,
}

/// Move a section; responds with `204 No Content` on success.
pub async fn move_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<MoveSectionRequest>,
) -> Response {
    match state.service.move_section_by_id(id, body.position).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
