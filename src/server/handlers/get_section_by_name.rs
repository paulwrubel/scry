//! Handler for fetching a single section by project and name.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a section by project id and name; a missing section is a `404`.
pub async fn get_section_by_name(
    State(state): State<ScryState>,
    Path((project_id, name)): Path<(ProjectId, String)>,
) -> Response {
    match state.service.find_section_by_name(project_id, &name).await {
        Ok(Some(section)) => Json(section).into_response(),
        Ok(None) => AppError::Storage(StorageError::NotFound(format!(
            "section '{name}' not found in project {project_id}"
        )))
        .into(),
        Err(error) => AppError::from(error).into(),
    }
}
