//! Handler for deleting an automation by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::AutomationId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete an automation by id and return it.
pub async fn delete_automation(
    State(state): State<ScryState>,
    Path(id): Path<AutomationId>,
) -> Response {
    match state.service.delete_automation(id).await {
        Ok(automation) => Json(automation).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
