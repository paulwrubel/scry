//! Handler for updating a task by handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use scry_core::models::{Priority, SectionId, Tags};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::{SectionInput, TaskPatch};

/// Request body for updating a task; omitted fields are left unchanged.
#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub section_id: Option<SectionId>,
    pub new_section: Option<String>,
    #[serde(default)]
    pub clear_section: bool,
}

/// Update a task from the request body.
pub async fn update_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Json(body): Json<UpdateTaskRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let section_choices = [
        body.section_id.is_some(),
        body.new_section.is_some(),
        body.clear_section,
    ];
    if section_choices.into_iter().filter(|chosen| *chosen).count() > 1 {
        return AppError::Usage(
            "specify at most one of section_id, new_section, or clear_section".to_string(),
        )
        .into();
    }

    let section = if let Some(section_id) = body.section_id {
        SectionInput::Existing(section_id)
    } else if let Some(name) = body.new_section {
        SectionInput::New(name)
    } else if body.clear_section {
        SectionInput::NoSection
    } else {
        SectionInput::Unchanged
    };

    let input = TaskPatch {
        title: body.title,
        description: body.description,
        priority: body.priority,
        tags: body.tags.map(Tags::from),
        section,
    };

    match state.service.update_task_by_id(task.id, input).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
