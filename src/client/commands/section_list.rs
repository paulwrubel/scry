//! `section list`.

use scry_core::models::{Project, Section};

use crate::client::Client;
use crate::color;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// List a project's sections.
pub async fn section_list(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let project_id = project.id.to_string();
    let sections: Vec<Section> = client
        .get_json(&["projects", project_id.as_str(), "sections"], &[])
        .await?;

    if output_json {
        print_json(&sections)?;
    } else {
        println!("Sections for \"{}\":", project.name);
        for section in &sections {
            anstream::println!("  {}", color::status(section.color, &section.name));
        }
    }

    Ok(())
}
