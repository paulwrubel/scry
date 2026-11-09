//! Handler for creating a section within a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Color, ProjectId};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a section.
#[derive(Deserialize)]
pub struct CreateSectionRequest {
    pub name: String,
    pub color: Option<Color>,
}

/// Create a section in a project and return it.
pub async fn create_section(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateSectionRequest>,
) -> Response {
    match state
        .service
        .create_section_in_project(project_id, body.name, body.color)
        .await
    {
        Ok(section) => Json(section).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
