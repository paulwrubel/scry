//! Handler for importing a snapshot file.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::snapshot::{self, ImportMode};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for an import: the raw snapshot file plus the collision mode.
#[derive(Deserialize)]
pub struct ImportRequest {
    pub snapshot: serde_json::Value,
    pub mode: ImportMode,
}

/// Parse a snapshot file and apply it server-side.
pub async fn import(State(state): State<ScryState>, Json(body): Json<ImportRequest>) -> Response {
    let text = match serde_json::to_string(&body.snapshot) {
        Ok(text) => text,
        Err(error) => {
            return AppError::Internal(format!("failed to serialize snapshot: {error}")).into();
        }
    };

    let file = match snapshot::parse_file(&text) {
        Ok(file) => file,
        Err(error) => return AppError::Usage(error.to_string()).into(),
    };

    match state.service.import(&file, body.mode).await {
        Ok(report) => Json(report).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
