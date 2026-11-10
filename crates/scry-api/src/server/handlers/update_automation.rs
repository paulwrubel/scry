//! Handler for updating an automation.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{AutomationAction, AutomationId};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for updating an automation.
#[derive(Deserialize)]
pub struct UpdateAutomationRequest {
    pub actions: Vec<AutomationAction>,
}

/// Update an automation's action list, keyed by the path id.
pub async fn update_automation(
    State(state): State<ScryState>,
    Path(id): Path<AutomationId>,
    Json(body): Json<UpdateAutomationRequest>,
) -> Response {
    match state.service.update_automation(id, body.actions).await {
        Ok(automation) => Json(automation).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
