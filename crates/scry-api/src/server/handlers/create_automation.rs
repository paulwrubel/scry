//! Handler for creating an automation in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{AutomationAction, AutomationTrigger, ProjectId};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating an automation.
#[derive(Deserialize)]
pub struct CreateAutomationRequest {
    pub trigger: AutomationTrigger,
    pub actions: Vec<AutomationAction>,
}

/// Create an automation in a project and return it.
pub async fn create_automation(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateAutomationRequest>,
) -> Response {
    match state
        .service
        .create_automation_in_project(project_id, body.trigger, body.actions)
        .await
    {
        Ok(automation) => Json(automation).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
