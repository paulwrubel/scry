//! Handler for reading the highest changes-log seq.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::error::AppError;
use crate::server::ScryState;

/// The highest changes-log `seq`, or `0` when the log is empty.
#[derive(Serialize)]
pub struct MaxSeq {
    pub max_seq: i64,
}

/// Return the server's current sync cursor without applying or pulling anything.
pub async fn max_seq(State(state): State<ScryState>) -> Response {
    match state.service.max_seq().await {
        Ok(max_seq) => Json(MaxSeq { max_seq }).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
