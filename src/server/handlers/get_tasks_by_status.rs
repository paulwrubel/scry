//! Handler for listing the tasks in a status.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::StatusId;

use crate::error::AppError;
use crate::server::ScryState;

/// List the tasks in a status.
pub async fn get_tasks_by_status(
    State(state): State<ScryState>,
    Path(status_id): Path<StatusId>,
) -> Response {
    match state.service.list_tasks_by_status(status_id).await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
