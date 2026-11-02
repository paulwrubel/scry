//! Handler for the bare server-wide snapshot.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::server::ScryState;

/// Return every project document plus the cursor to pull from next.
pub async fn snapshot(State(state): State<ScryState>) -> Response {
    match state.service.snapshot(None).await {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
