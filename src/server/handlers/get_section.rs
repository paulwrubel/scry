//! Handler for fetching a single section by id.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::models::SectionId;

use crate::error::AppError;
use crate::server::ScryState;

/// Fetch a section by id; a missing section is a `404`.
pub async fn get_section(State(state): State<ScryState>, Path(id): Path<SectionId>) -> Response {
    match state.service.get_section(id).await {
        Ok(Some(section)) => Json(section).into_response(),
        Ok(None) => {
            AppError::Storage(StorageError::NotFound(format!("section {id} not found"))).into()
        }
        Err(error) => AppError::from(error).into(),
    }
}
