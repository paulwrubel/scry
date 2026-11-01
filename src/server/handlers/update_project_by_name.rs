//! Handler for updating a project by name.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;

use crate::error::AppError;
use crate::server::ScryState;

use super::update_project::UpdateProjectRequest;

/// Replace a project's mutable fields by exact name.
pub async fn update_project_by_name(
    State(state): State<ScryState>,
    Path(name): Path<String>,
    Json(body): Json<UpdateProjectRequest>,
) -> Response {
    let existing = match state.service.get_project_by_name(&name).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!(
                "project '{name}' not found"
            )))
            .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.update_project(&existing, body.into()).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
