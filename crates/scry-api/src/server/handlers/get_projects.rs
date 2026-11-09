//! Handler for listing all projects.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// List every project.
pub async fn get_projects(State(state): State<ScryState>) -> Response {
    match state.service.list_projects().await {
        Ok(projects) => Json(projects).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
