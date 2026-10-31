//! Handler for moving a task to a section and/or position.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{SectionId, TaskId};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::SectionInput;

/// Request body for moving a task; an omitted `section_id` means no section.
#[derive(Deserialize)]
pub struct MoveTaskRequest {
    pub section_id: Option<SectionId>,
    pub position: Option<i32>,
}

/// Move a task into a section (or no section) at an optional position.
pub async fn move_task(
    State(state): State<ScryState>,
    Path(id): Path<TaskId>,
    Json(body): Json<MoveTaskRequest>,
) -> Response {
    let section = match body.section_id {
        Some(section_id) => SectionInput::Existing(section_id),
        None => SectionInput::NoSection,
    };

    match state
        .service
        .move_task_by_id(id, section, body.position)
        .await
    {
        Ok(change) => Json(change.task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
