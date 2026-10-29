//! Handler for listing all statuses in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List every status in a project.
pub async fn get_statuses(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_statuses(project_id).await {
        Ok(statuses) => Json(statuses).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
