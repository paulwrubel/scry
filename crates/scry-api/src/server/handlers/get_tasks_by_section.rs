//! Handler for listing the tasks in a section.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the tasks in a section.
pub async fn get_tasks_by_section(
    State(state): State<ScryState>,
    Path(section_id): Path<SectionId>,
) -> Response {
    match state.service.list_tasks_by_section_id(section_id).await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
