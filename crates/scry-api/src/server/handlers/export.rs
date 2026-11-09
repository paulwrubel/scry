//! Handler for exporting every project as a snapshot file.

use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Query options for an export.
#[derive(Debug, Default, Deserialize)]
pub struct ExportQuery {
    /// Include the snapshot's cursor in the file.
    #[serde(default)]
    pub include_seq: bool,
}

/// Export every project as one importable snapshot file.
pub async fn export(State(state): State<ScryState>, Query(query): Query<ExportQuery>) -> Response {
    match state.service.snapshot_file(None, query.include_seq).await {
        Ok(mut file) => {
            file.app_version = env!("CARGO_PKG_VERSION").to_string();
            Json(file).into_response()
        }
        Err(error) => AppError::from(error).into(),
    }
}
