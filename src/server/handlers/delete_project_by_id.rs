//! Handler for deleting a project by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a project by id; responds with `204 No Content` on success.
pub async fn delete_project_by_id(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
) -> Response {
    let name = match state.store.get_project_by_id(id).await {
        Ok(Some(project)) => project.name,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!("project {id} not found")))
                .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.store.delete_project(name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
