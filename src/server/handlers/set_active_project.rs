//! Handler for setting the active project.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for setting the active project.
#[derive(Deserialize)]
pub struct SetActiveProjectRequest {
    pub name: String,
}

/// Set the active project; responds with `204 No Content` on success.
pub async fn set_active_project(
    State(state): State<ScryState>,
    Json(body): Json<SetActiveProjectRequest>,
) -> Response {
    match state.store.set_active_project(&body.name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
