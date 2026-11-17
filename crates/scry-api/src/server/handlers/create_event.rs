//! Handler for creating an event in a project.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::ProjectId;
use scry_core::temporal::Interval;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for creating an event.
#[derive(Deserialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub interval: Interval,
}

/// Create an event in a project and return it.
pub async fn create_event(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateEventRequest>,
) -> Response {
    match state
        .service
        .create_event_in_project(
            project_id,
            crate::service::NewEvent {
                title: body.title,
                description: body.description,
                interval: body.interval,
            },
        )
        .await
    {
        Ok(event) => Json(event).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
