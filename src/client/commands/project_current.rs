//! `project current`.

use scry_core::models::Project;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// Show the active project.
pub async fn project_current(
    client: &Client,
    config: &ScryConfig,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, None).await?;

    if output_json {
        print_json(&project)?;
    } else {
        println!("{}", project.name);
    }

    Ok(())
}
