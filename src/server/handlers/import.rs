//! Handler for applying an import plan.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::backup::ImportPlan;

use crate::error::AppError;
use crate::server::ScryState;

/// Apply an import plan atomically and return the import report.
pub async fn import(State(state): State<ScryState>, Json(plan): Json<ImportPlan>) -> Response {
    match state.store.import(plan).await {
        Ok(report) => Json(report).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
