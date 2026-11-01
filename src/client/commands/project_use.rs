//! `project use`.

use scry_core::models::Project;

use crate::client::Client;
use crate::client_state::ClientState;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// Make a project the active project.
pub async fn project_use(
    client: &Client,
    config: &ScryConfig,
    name: &str,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, Some(name)).await?;

    ClientState::set_active(config, &project.name)?;

    if output_json {
        print_json(&project)?;
    } else {
        println!("Using project \"{}\"", project.name);
    }

    Ok(())
}
