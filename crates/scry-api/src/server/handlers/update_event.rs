//! Handler for updating an event by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::EventId;
use scry_core::models::EventOverride;
use scry_core::recurrence::RRule;
use scry_core::temporal::Interval;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::EventPatch;

/// Request body for updating an event. An absent field is unchanged; an empty
/// description clears it; `recurrence` absent leaves it unchanged, `null`
/// clears it, and a rule sets it. `overrides` absent leaves them unchanged and
/// a list replaces the whole set.
#[derive(Deserialize)]
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub interval: Option<Interval>,
    #[serde(default, with = "scry_core::sync::double_option")]
    pub recurrence: Option<Option<RRule>>,
    #[serde(default)]
    pub overrides: Option<Vec<EventOverride>>,
}

/// Update an event and return it.
pub async fn update_event(
    State(state): State<ScryState>,
    Path(id): Path<EventId>,
    Json(body): Json<UpdateEventRequest>,
) -> Response {
    match state
        .service
        .update_event(
            id,
            EventPatch {
                title: body.title,
                description: body.description,
                interval: body.interval,
                recurrence: body.recurrence,
                overrides: body.overrides,
            },
        )
        .await
    {
        Ok(event) => Json(event).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
