//! `duplicate`.

use crate::client::{Client, TaskDetails};
use crate::error::AppError;
use crate::output::print_json;

/// Duplicate a task.
pub async fn duplicate(client: &Client, handle: &str, output_json: bool) -> Result<(), AppError> {
    let detail: TaskDetails = client
        .post_json(&["tasks", handle, "duplicate"], &[], &serde_json::json!({}))
        .await?;

    if output_json {
        print_json(&detail.task)?;
    } else {
        let new_handle = detail.handle.as_deref().unwrap_or("?");
        println!("Duplicated task {} as task {}.", handle, new_handle);
    }

    Ok(())
}
