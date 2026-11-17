//! `close`.

use super::render_batch_results;
use crate::client::{BatchTaskResult, Client};
use crate::error::AppError;
use scry_core::ids::Handle;
use scry_core::models::ClosedReason;
use serde::Serialize;

/// Close several tasks in one request.
pub async fn close_tasks(
    client: &Client,
    handles: &[Handle],
    reason: ClosedReason,
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct CloseTasksRequest<'a> {
        handles: Vec<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<ClosedReason>,
    }

    let body = CloseTasksRequest {
        handles: handles.iter().map(Handle::as_str).collect(),
        // an explicit completed close still needs to reach the server
        reason: Some(reason),
    };

    let results: Vec<BatchTaskResult> = client.post_json(&["tasks", "close"], &[], &body).await?;

    render_batch_results(&results, "Closed", "close", output_json)
}
