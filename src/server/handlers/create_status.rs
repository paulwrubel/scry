//! Handler for creating a status within a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::{Color, ProjectId, StatusStyle};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating a status; omitted fields fall back to defaults.
#[derive(Deserialize)]
pub struct CreateStatusRequest {
    pub name: String,
    pub position: Option<i32>,
    pub color: Option<Color>,
    pub style: Option<StatusStyle>,
}

/// Create a status in a project and return it.
pub async fn create_status(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateStatusRequest>,
) -> Response {
    match state
        .store
        .create_status(
            project_id,
            body.name,
            body.position.unwrap_or(0),
            body.color,
            body.style.unwrap_or_default(),
        )
        .await
    {
        Ok(status) => Json(status).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
