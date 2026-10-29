//! Handler for deleting a project by id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use scry_core::models::ProjectId;

use crate::error::AppError;
use crate::server::ScryState;

/// Delete a project by id; responds with `204 No Content` on success.
pub async fn delete_project_by_id(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
) -> Response {
    match state.service.delete_project_by_id(id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
