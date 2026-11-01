//! `close`.

use super::render_batch_results;
use crate::client::{BatchTaskResult, Client};
use crate::error::AppError;
use scry_core::ids::Handle;
use serde::Serialize;

/// Close several tasks in one request.
pub async fn close_tasks(
    client: &Client,
    handles: &[Handle],
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct CloseTasksRequest<'a> {
        handles: Vec<&'a str>,
    }

    let body = CloseTasksRequest {
        handles: handles.iter().map(Handle::as_str).collect(),
    };

    let results: Vec<BatchTaskResult> = client.post_json(&["tasks", "close"], &[], &body).await?;

    render_batch_results(&results, "Closed", "close", output_json)
}
