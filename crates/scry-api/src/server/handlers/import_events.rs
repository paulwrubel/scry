//! Handler for importing iCalendar events into a project.

use crate::error::AppError;
use crate::server::ScryState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use scry_core::ids::ProjectId;

/// Import a VCALENDAR into a project and return the created events.
pub async fn import_events(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let is_calendar = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/calendar"));
    if !is_calendar {
        return AppError::Usage("expected a text/calendar body".to_string()).into();
    }

    match state.service.import_ics(project_id, &body).await {
        Ok(events) => Json(events).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
