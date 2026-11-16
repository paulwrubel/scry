//! Handler for updating an event by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::EventId;
use scry_core::temporal::Interval;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::EventPatch;

/// Request body for updating an event. An absent field is unchanged; an empty
/// description clears it.
#[derive(Deserialize)]
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub interval: Option<Interval>,
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
            },
        )
        .await
    {
        Ok(event) => Json(event).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
