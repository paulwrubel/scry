//! Handler for listing the notes on a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;

use crate::error::AppError;
use crate::server::ScryState;

/// List the notes on a task.
pub async fn get_notes_by_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.list_notes_by_task_id(task.id).await {
        Ok(notes) => Json(notes).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
