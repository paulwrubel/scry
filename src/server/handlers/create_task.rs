//! Handler for creating a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Priority, ProjectId, SectionId, Tags};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::{SectionInput, TaskInput};

/// Request body for creating a task; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub section_id: Option<SectionId>,
    pub new_section: Option<String>,
}

/// Create a task in a project and return it.
pub async fn create_task(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateTaskRequest>,
) -> Response {
    let section = match (body.section_id, body.new_section) {
        (Some(_), Some(_)) => {
            return AppError::Usage("specify at most one of section_id or new_section".to_string())
                .into();
        }
        (Some(section_id), None) => SectionInput::Existing(section_id),
        (None, Some(name)) => SectionInput::New(name),
        (None, None) => SectionInput::NoSection,
    };

    let input = TaskInput {
        title: Some(body.title),
        description: body.description,
        priority: body.priority,
        tags: body.tags.map(Tags::from),
        section,
    };

    match state
        .service
        .create_task_in_project(project_id, input)
        .await
    {
        Ok(change) => Json(change.task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
