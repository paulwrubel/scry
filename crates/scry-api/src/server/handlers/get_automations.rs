//! Handler for listing automations in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// List every automation in a project.
pub async fn get_automations(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.list_automations(project_id).await {
        Ok(automations) => Json(automations).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
