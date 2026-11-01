//! `close`.

use scry_core::ids::Handle;
use scry_core::models::CloseReason;
use serde::Serialize;

use crate::client::{BatchTaskResult, Client};
use crate::error::AppError;

use super::render_batch_results;

/// Close several tasks in one request.
pub async fn close_tasks(
    client: &Client,
    handles: &[Handle],
    reason: CloseReason,
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct CloseTasksRequest<'a> {
        handles: Vec<&'a str>,
        reason: CloseReason,
    }

    let body = CloseTasksRequest {
        handles: handles.iter().map(Handle::as_str).collect(),
        reason,
    };

    let results: Vec<BatchTaskResult> = client
        .post_json(&["tasks", "close"], &[], &body)
        .await?;

    render_batch_results(&results, "Closed", "close", output_json)
}
