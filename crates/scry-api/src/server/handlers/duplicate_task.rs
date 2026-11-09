//! Handler for duplicating a task.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::ids::Handle;

use crate::error::AppError;
use crate::server::ScryState;

use super::TaskDetails;

/// Duplicate a task and return the new task with its handle.
pub async fn duplicate_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
) -> Response {
    let source = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let project = match state.service.get_project(source.project_id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            return AppError::Storage(StorageError::NotFound(format!(
                "project {} not found",
                source.project_id
            )))
            .into();
        }
        Err(error) => return AppError::from(error).into(),
    };

    match state.service.duplicate_task(&project, source.id).await {
        Ok(change) => {
            let handle = match state.service.get_task_handle(change.task.id).await {
                Ok(handle) => handle,
                Err(error) => return AppError::from(error).into(),
            };

            Json(TaskDetails {
                task: change.task,
                notes: None,
                handle: Some(handle),
            })
            .into_response()
        }
        Err(error) => AppError::from(error).into(),
    }
}
