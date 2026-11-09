//! Handler for listing all sections in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List every section in a project.
pub async fn get_sections(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_sections_by_project_id(project_id).await {
        Ok(sections) => Json(sections).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
