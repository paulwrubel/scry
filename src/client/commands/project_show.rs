//! `project show`.

use scry_core::models::Project;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;
use crate::state::format_local;

use super::project_by_name;

/// Show a project's details.
pub async fn project_show(
    client: &Client,
    config: &ScryConfig,
    name: Option<&str>,
    project_flag: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, name.or(project_flag)).await?;

    if output_json {
        print_json(&project)?;
        return Ok(());
    }

    println!("{}", project.name);
    if let Some(description) = &project.description {
        for line in description.lines() {
            println!("    {}", line);
        }
    }
    println!();
    println!("Created at:   {}", format_local(project.created_at));
    println!("Updated at:   {}", format_local(project.updated_at));

    Ok(())
}
