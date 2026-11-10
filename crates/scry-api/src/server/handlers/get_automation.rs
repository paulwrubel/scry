//! Handler for fetching a single automation by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::AutomationId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch an automation by id; a missing automation is a `404`.
pub async fn get_automation(
    State(state): State<ScryState>,
    Path(id): Path<AutomationId>,
) -> Response {
    match state.service.get_automation(id).await {
        Ok(Some(automation)) => Json(automation).into_response(),
        Ok(None) => {
            AppError::Storage(StorageError::NotFound(format!("automation {id} not found"))).into()
        }
        Err(error) => AppError::from(error).into(),
    }
}
