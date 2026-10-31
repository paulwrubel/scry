//! Handler for updating a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::{ProjectId, TaskSortingMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::UpdateProject;

/// Request body for replacing a project's mutable fields.
#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
}

impl From<UpdateProjectRequest> for UpdateProject {
    fn from(request: UpdateProjectRequest) -> Self {
        Self {
            name: Some(request.name),
            // An absent or null description keeps PUT's full-replace behavior:
            // it clears the field rather than leaving it alone.
            description: Some(request.description.unwrap_or_default()),
            task_sorting_mode: Some(request.task_sorting_mode),
            show_priority: Some(request.show_priority),
        }
    }
}

/// Replace a project's mutable fields, preserving its id and creation time.
pub async fn update_project(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
    Json(body): Json<UpdateProjectRequest>,
) -> Response {
    let existing = match state.service.get_project_by_id(id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!("project {id} not found")))
                .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.update_project(&existing, body.into()).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
