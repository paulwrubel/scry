//! `project delete`.

use std::io::Write;

use scry_core::models::Project;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Delete a project.
pub async fn project_delete(
    client: &Client,
    name: &str,
    force: bool,
    output_json: bool,
) -> Result<(), AppError> {
    if !force && !output_json {
        print!("Delete project \"{}\" and all its tasks? [y/N]: ", name);
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let project: Project = client
        .delete_json(&["projects", "by-name", name], &[])
        .await?;

    if output_json {
        print_json(&project)?;
    } else {
        println!("Deleted project \"{}\"", project.name);
    }

    Ok(())
}
