//! Handler for updating a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::{Project, ProjectId, TaskSortingMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for replacing a project's mutable fields.
#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
}

/// Replace a project, preserving its id and creation time.
pub async fn update_project(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
    Json(body): Json<UpdateProjectRequest>,
) -> Response {
    let existing = match state.service.find_project(id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!("project {id} not found")))
                .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    let project = Project {
        id,
        name: body.name,
        description: body.description,
        task_sorting_mode: body.task_sorting_mode,
        show_priority: body.show_priority,
        created_at: existing.created_at,
    };

    match state.service.replace_project(project).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
