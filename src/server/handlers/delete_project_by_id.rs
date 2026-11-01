//! Handler for deleting a project by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a project by id; responds with `204 No Content`.
pub async fn delete_project_by_id(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
) -> Response {
    let project = match state.service.get_project(id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!("project '{id}' not found")))
                .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.delete_project_by_id(project.id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
