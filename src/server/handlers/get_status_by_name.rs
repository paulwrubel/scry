//! Handler for fetching a single status by project and name.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a status by project id and name; responds with `null` when absent.
pub async fn get_status_by_name(
    State(state): State<ScryState>,
    Path((project_id, status_name)): Path<(ProjectId, String)>,
) -> Response {
    match state
        .store
        .get_status_by_project_id_and_status_name(project_id, status_name)
        .await
    {
        Ok(status) => Json(status).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
