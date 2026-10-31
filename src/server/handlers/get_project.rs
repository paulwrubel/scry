//! Handler for fetching a single project by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a project by id; responds with `null` when it does not exist.
pub async fn get_project(State(state): State<ScryState>, Path(id): Path<ProjectId>) -> Response {
    match state.service.get_project_by_id(id).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
