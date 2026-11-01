//! Handler for exporting one project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Export one project as a backup.
pub async fn export_project(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.export(Some(project_id)).await {
        Ok(mut backup) => {
            backup.app_version = env!("CARGO_PKG_VERSION").to_string();
            Json(backup).into_response()
        }
        Err(error) => AppError::from(error).into(),
    }
}
