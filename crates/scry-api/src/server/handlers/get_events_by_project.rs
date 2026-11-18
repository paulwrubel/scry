//! Handler for listing the events in a project.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::ids::ProjectId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::EventWindow;

/// Optional instant window. Both bounds are RFC 3339 timestamps and must be
/// supplied together.
#[derive(Deserialize)]
pub struct EventListQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// List the events in a project, ordered by start.
pub async fn get_events_by_project(
    State(state): State<ScryState>,
    Path(project_id): Path<ProjectId>,
    Query(query): Query<EventListQuery>,
    headers: HeaderMap,
) -> Response {
    let window = match parse_window(&query) {
        Ok(window) => window,
        Err(message) => return AppError::Usage(message).into(),
    };

    match state
        .service
        .list_events(project_id, window, &request_zone(&headers))
        .await
    {
        Ok(events) => Json(events).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}

fn parse_window(query: &EventListQuery) -> Result<Option<EventWindow>, String> {
    match (&query.from, &query.to) {
        (None, None) => Ok(None),
        (Some(from), Some(to)) => {
            let from = from
                .parse::<Timestamp>()
                .map_err(|error| format!("invalid from timestamp: {error}"))?;
            let to = to
                .parse::<Timestamp>()
                .map_err(|error| format!("invalid to timestamp: {error}"))?;
            if to <= from {
                return Err("to must be after from".to_string());
            }
            Ok(Some(EventWindow { from, to }))
        }
        _ => Err("from and to must be provided together".to_string()),
    }
}

/// The viewer's zone, used to turn a date or floating start into an instant.
pub(crate) fn request_zone(headers: &HeaderMap) -> TimeZone {
    headers
        .get("time-zone")
        .and_then(|value| value.to_str().ok())
        .and_then(|name| TimeZone::get(name).ok())
        .unwrap_or(TimeZone::UTC)
}
