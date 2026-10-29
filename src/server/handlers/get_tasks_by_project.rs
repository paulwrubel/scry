//! Handler for listing the tasks in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the tasks in a project.
pub async fn get_tasks_by_project(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_tasks_by_project(project_id).await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
