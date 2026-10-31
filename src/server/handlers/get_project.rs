//! Handler for fetching a single project by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a project by id; a missing project is a `404`.
pub async fn get_project(State(state): State<ScryState>, Path(id): Path<ProjectId>) -> Response {
    match state.service.get_project(id).await {
        Ok(Some(project)) => Json(project).into_response(),
        Ok(None) => {
            AppError::Storage(StorageError::NotFound(format!("project {id} not found"))).into()
        }
        Err(error) => AppError::from(error).into(),
    }
}
