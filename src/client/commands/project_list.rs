//! `project list`.

use scry_core::models::Project;

use crate::client::Client;
use crate::client_state::ClientState;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

/// List all projects.
pub async fn project_list(
    client: &Client,
    config: &ScryConfig,
    output_json: bool,
) -> Result<(), AppError> {
    let projects: Vec<Project> = client.get_json(&["projects"], &[]).await?;

    if output_json {
        print_json(&projects)?;
    } else if projects.is_empty() {
        eprintln!("No projects. Run 'scry project create <name>' to create one.");
    } else {
        let active = ClientState::load(config)?
            .active_project
            .unwrap_or_else(|| "default".to_string());

        for project in &projects {
            let marker = if project.name == active { "* " } else { "  " };
            println!("  {}{}", marker, project.name);
        }
    }

    Ok(())
}
