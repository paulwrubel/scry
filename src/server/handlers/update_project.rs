//! Handler for updating a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Project, ProjectId};

use crate::error::AppError;
use crate::server::ScryState;

/// Update a project; the id is taken from the path, not the body.
pub async fn update_project(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
    Json(mut project): Json<Project>,
) -> Response {
    project.id = id;
    project.description = project
        .description
        .filter(|description| !description.is_empty());
    match state.store.update_project(project).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
