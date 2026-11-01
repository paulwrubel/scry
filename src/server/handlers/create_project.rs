//! Handler for creating a project.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::models::{PROJECT_TEMPLATES, TaskSortingMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::ProjectSetup;

/// Request body for creating a project; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
    pub template: Option<String>,
}

/// Create a project and return it.
pub async fn create_project(
    State(state): State<ScryState>,
    Json(body): Json<CreateProjectRequest>,
) -> Response {
    let setup = match body.template {
        Some(name) => {
            let Some(template) = PROJECT_TEMPLATES
                .iter()
                .find(|template| template.name == name)
            else {
                return AppError::Usage(format!("Unknown template name: \"{name}\".")).into();
            };

            ProjectSetup::Template(*template)
        }
        None => ProjectSetup::Explicit {
            task_sorting_mode: body.task_sorting_mode.unwrap_or_default(),
            show_priority: body.show_priority.unwrap_or_default(),
        },
    };

    match state
        .service
        .create_project(body.name, body.description, setup)
        .await
    {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
