//! Handler for exporting a project's events as iCalendar.

use crate::error::AppError;
use crate::server::ScryState;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use scry_core::ids::ProjectId;

/// Export a project's events as a VCALENDAR document.
pub async fn export_events(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
) -> Response {
    match state.service.export_ics(project_id).await {
        Ok(calendar) => {
            ([(CONTENT_TYPE, "text/calendar; charset=utf-8")], calendar).into_response()
        }
        Err(error) => AppError::from(error).into(),
    }
}
