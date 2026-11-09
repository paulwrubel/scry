//! Handler for deleting a section by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a section by id and return it.
pub async fn delete_section(State(state): State<ScryState>, Path(id): Path<SectionId>) -> Response {
    match state.service.delete_section_by_id(id).await {
        Ok(section) => Json(section).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
