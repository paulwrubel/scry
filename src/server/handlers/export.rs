//! Handler for exporting all data.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// Export every project as a backup.
pub async fn export(State(state): State<ScryState>) -> Response {
    match state.service.export(None).await {
        Ok(mut backup) => {
            backup.app_version = env!("CARGO_PKG_VERSION").to_string();
            Json(backup).into_response()
        }
        Err(error) => AppError::from(error).into(),
    }
}
