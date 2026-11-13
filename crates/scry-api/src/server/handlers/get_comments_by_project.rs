//! Handler for listing the comments in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the comments in a project.
pub async fn get_comments_by_project(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_comments_by_project_id(project_id).await {
        Ok(comments) => Json(comments).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
