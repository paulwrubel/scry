//! Handler for closing a task by handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use scry_core::models::CloseReason;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;

/// Request body for closing a task.
#[derive(Deserialize)]
pub struct CloseTaskRequest {
    pub reason: Option<CloseReason>,
}

/// Close a task and return it.
pub async fn close_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Json(body): Json<CloseTaskRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let Some(reason) = body.reason else {
        return AppError::Usage("a close reason is required: completed or cancelled".to_string())
            .into();
    };

    match state.service.close_task_by_id(task.id, reason).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}
