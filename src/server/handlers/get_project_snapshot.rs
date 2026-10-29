//! Handler for fetching a project snapshot.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a project snapshot: the project with all of its statuses, tasks, and notes.
pub async fn get_project_snapshot(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.project_snapshot(project_id).await {
        Ok(project_snapshot) => Json(project_snapshot).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
