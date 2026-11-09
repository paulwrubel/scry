//! Handler for listing the notes in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the notes in a project.
pub async fn get_notes_by_project(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_notes_by_project_id(project_id).await {
        Ok(notes) => Json(notes).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
