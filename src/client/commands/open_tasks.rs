//! `open`.

use scry_core::ids::Handle;
use serde::Serialize;

use crate::client::{BatchTaskResult, Client};
use crate::error::AppError;

use super::render_batch_results;

/// Open several tasks in one request.
pub async fn open_tasks(
    client: &Client,
    handles: &[Handle],
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct OpenTasksRequest<'a> {
        handles: Vec<&'a str>,
    }

    let body = OpenTasksRequest {
        handles: handles.iter().map(Handle::as_str).collect(),
    };

    let results: Vec<BatchTaskResult> = client
        .post_json(&["tasks", "open"], &[], &body)
        .await?;

    render_batch_results(&results, "Opened", "open", output_json)
}
