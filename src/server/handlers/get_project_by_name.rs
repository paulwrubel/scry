//! Handler for fetching a single project by name.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a project by name; responds with `null` when it does not exist.
pub async fn get_project_by_name(
    State(state): State<ScryState>,
    Path(name): Path<String>,
) -> Response {
    match state.store.get_project_by_name(&name).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
