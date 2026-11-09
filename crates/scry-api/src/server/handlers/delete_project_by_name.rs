//! Handler for deleting a project by name.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a project by exact name and return the removed project.
pub async fn delete_project_by_name(
    State(state): State<ScryState>,
    Path(name): Path<String>,
) -> Response {
    let project = match state.service.get_project_by_name(&name).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!(
                "project '{name}' not found"
            )))
            .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.delete_project_by_id(project.id).await {
        Ok(removed) => Json(removed).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
