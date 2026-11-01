//! Handler for importing a backup.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::backup::{self, ImportMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for an import: the raw backup plus the collision mode.
#[derive(Deserialize)]
pub struct ImportRequest {
    pub backup: serde_json::Value,
    pub mode: ImportMode,
}

/// Parse a backup, plan the import, and apply it server-side.
pub async fn import(State(state): State<ScryState>, Json(body): Json<ImportRequest>) -> Response {
    let text = match serde_json::to_string(&body.backup) {
        Ok(text) => text,
        Err(error) => {
            return AppError::Internal(format!("failed to serialize backup: {error}")).into();
        }
    };

    let backup = match backup::parse(&text) {
        Ok(backup) => backup,
        Err(error) => return AppError::Usage(error.to_string()).into(),
    };

    let plan = match state.service.plan_import(&backup, body.mode).await {
        Ok(plan) => plan,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.import(plan).await {
        Ok(report) => Json(report).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
