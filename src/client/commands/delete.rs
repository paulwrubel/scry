//! `delete`.

use scry_core::models::Task;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Delete a task.
pub async fn delete(client: &Client, handle: &str, output_json: bool) -> Result<(), AppError> {
    let task: Task = client.delete_json(&["tasks", handle], &[]).await?;

    if output_json {
        print_json(&task)?;
    } else {
        println!("Deleted task {}.", handle);
    }

    Ok(())
}
