//! Handler for creating a project.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::models::{StatusId, TaskSortingMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a project; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub entry_status_id: Option<StatusId>,
    pub task_sorting_mode: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
}

/// Create a project and return it.
pub async fn create_project(
    State(state): State<ScryState>,
    Json(body): Json<CreateProjectRequest>,
) -> Response {
    match state
        .store
        .create_project(
            body.name,
            body.entry_status_id,
            body.task_sorting_mode.unwrap_or_default(),
            body.show_priority.unwrap_or_default(),
        )
        .await
    {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
