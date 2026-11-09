//! Handler for moving a section to a new position.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::SectionPlacement;

/// Request body for moving a section; the `op` tag selects the placement.
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MoveSectionRequest {
    Top,
    Bottom,
    Before { anchor_id: SectionId },
    After { anchor_id: SectionId },
}

/// Move a section and return the updated section.
pub async fn move_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<MoveSectionRequest>,
) -> Response {
    let placement = match body {
        MoveSectionRequest::Top => SectionPlacement::Top,
        MoveSectionRequest::Bottom => SectionPlacement::Bottom,
        MoveSectionRequest::Before { anchor_id } => SectionPlacement::Before(anchor_id),
        MoveSectionRequest::After { anchor_id } => SectionPlacement::After(anchor_id),
    };

    match state.service.move_section(id, placement).await {
        Ok(section) => Json(section).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
