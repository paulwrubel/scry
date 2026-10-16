//! Handler for fetching the active project.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch the currently active project.
pub async fn get_active_project(State(state): State<ScryState>) -> Response {
    match state.store.get_active_project().await {
        Ok(project) => Json(project).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
