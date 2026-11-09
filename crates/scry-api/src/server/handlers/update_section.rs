//! Handler for updating a section.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Color, SectionId};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::{ColorChange, UpdateSection};

/// Request body for updating a section.
#[derive(Deserialize)]
pub struct UpdateSectionRequest {
    pub name: String,
    pub color: Option<Color>,
}

/// Update a section from the request body, keyed by the path id.
pub async fn update_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<UpdateSectionRequest>,
) -> Response {
    match state
        .service
        .update_section(
            id,
            UpdateSection {
                name: Some(body.name),
                // PUT replaces the row, so an omitted color clears it.
                color: body.color.map_or(ColorChange::Clear, ColorChange::Set),
            },
        )
        .await
    {
        Ok(section) => Json(section).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
