//! Handler for closing several tasks in one request.

use super::batch::BatchTaskResult;
use crate::error::AppError;
use crate::server::ScryState;
use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use serde::Deserialize;

/// Request body for a batch close.
#[derive(Deserialize)]
pub struct CloseTasksRequest {
    pub handles: Vec<String>,
}

/// Close every handle in the request, reporting one outcome per handle.
pub async fn close_tasks(
    State(state): State<ScryState>,
    Json(body): Json<CloseTasksRequest>,
) -> Response {
    let mut results = Vec::with_capacity(body.handles.len());

    for raw in body.handles {
        let result = match raw.parse::<Handle>() {
            Ok(handle) => match state.service.get_task_by_handle(&handle).await {
                Ok(task) => match state.service.close_task_by_id(task.id).await {
                    Ok(task) => BatchTaskResult::success(raw.clone(), task),
                    Err(error) => BatchTaskResult::failure(raw.clone(), AppError::from(error)),
                },
                Err(error) => BatchTaskResult::failure(raw.clone(), AppError::from(error)),
            },
            Err(error) => BatchTaskResult::failure(raw.clone(), AppError::Usage(error.to_string())),
        };

        results.push(result);
    }

    Json(results).into_response()
}
