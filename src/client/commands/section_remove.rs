//! `section remove`.

use scry_core::models::{Project, Section};

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Remove a section from a project.
pub async fn section_remove(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    name: &str,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let section = section_by_name(client, &project, name).await?;
    let section_id = section.id.to_string();

    let removed: Section = client
        .delete_json(&["sections", section_id.as_str()], &[])
        .await?;

    if output_json {
        print_json(&removed)?;
    } else {
        println!(
            "Removed section \"{}\" from project \"{}\"; its tasks moved to No section",
            removed.name, project.name
        );
    }

    Ok(())
}
