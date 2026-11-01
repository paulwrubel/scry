//! `section add`.

use scry_core::models::{Color, Project, Section};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// Add a section to a project.
pub async fn section_add(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    name: &str,
    color: Option<Color>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;

    #[derive(Serialize)]
    struct CreateSectionRequest<'a> {
        name: &'a str,
        color: Option<Color>,
    }

    let project_id = project.id.to_string();
    let section: Section = client
        .post_json(
            &["projects", project_id.as_str(), "sections"],
            &[],
            &CreateSectionRequest { name, color },
        )
        .await?;

    if output_json {
        print_json(&section)?;
    } else {
        println!(
            "Added section \"{}\" to project \"{}\"",
            section.name, project.name
        );
    }

    Ok(())
}
