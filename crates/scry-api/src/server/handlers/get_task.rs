//! Handler for fetching a single task by handle.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

use super::TaskDetails;

/// Query options for the task detail endpoint.
#[derive(Deserialize, Default)]
pub struct GetTaskQuery {
    /// Include the task's notes.
    #[serde(default)]
    pub detailed: bool,
    /// Include the task's display handle.
    #[serde(default)]
    pub include_handle: bool,
}

/// Fetch a task by handle, optionally with its notes and display handle.
pub async fn get_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Query(query): Query<GetTaskQuery>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let notes = if query.detailed {
        match state.service.list_notes_by_task_id(task.id).await {
            Ok(notes) => Some(notes),
            Err(error) => return AppError::from(error).into(),
        }
    } else {
        None
    };

    let display_handle = if query.include_handle {
        match state.service.get_task_handle(task.id).await {
            Ok(handle) => Some(handle),
            Err(error) => return AppError::from(error).into(),
        }
    } else {
        None
    };

    Json(TaskDetails {
        task,
        notes,
        handle: display_handle,
    })
    .into_response()
}
