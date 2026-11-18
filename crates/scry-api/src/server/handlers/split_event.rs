//! Handler for splitting a recurring event before one occurrence.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use scry_core::ids::EventId;
use scry_core::temporal::{Interval, When};
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::server::handlers::get_events_by_project::request_zone;
use crate::service::FollowingEvent;

/// Request body for splitting a series before `original_start`. The following
/// series takes `title`, `description`, and `interval`.
#[derive(Deserialize)]
pub struct SplitEventRequest {
    pub original_start: When,
    pub title: String,
    pub description: Option<String>,
    pub interval: Interval,
}

/// Split a recurring event and return both resulting series.
pub async fn split_event(
    State(state): State<ScryState>,
    Path(id): Path<EventId>,
    headers: HeaderMap,
    Json(body): Json<SplitEventRequest>,
) -> Response {
    match state
        .service
        .split_event(
            id,
            body.original_start,
            FollowingEvent {
                title: body.title,
                description: body.description,
                interval: body.interval,
            },
            &request_zone(&headers),
        )
        .await
    {
        Ok(split) => Json(split).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
