//! Handler for merging one section into another.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for merging a section into a target section.
#[derive(Deserialize)]
pub struct MergeSectionRequest {
    pub target_id: SectionId,
}

/// Merge a section into a target and return the target section.
pub async fn merge_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<MergeSectionRequest>,
) -> Response {
    match state.service.merge_sections_by_id(id, body.target_id).await {
        Ok((_, target)) => Json(target).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
