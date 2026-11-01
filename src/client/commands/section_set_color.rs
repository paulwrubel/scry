//! `section set-color`.

use scry_core::models::{Color, Project, Section};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Set a section's color.
pub async fn section_set_color(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    name: &str,
    color: Color,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let section = section_by_name(client, &project, name).await?;

    #[derive(Serialize)]
    struct UpdateSectionRequest<'a> {
        name: &'a str,
        color: Option<Color>,
    }

    let section_id = section.id.to_string();
    let updated: Section = client
        .put_json(
            &["sections", section_id.as_str()],
            &[],
            &UpdateSectionRequest {
                name: &section.name,
                color: Some(color),
            },
        )
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        println!(
            "Set color of section \"{}\" to \"{}\" in project \"{}\"",
            name, color, project.name
        );
    }

    Ok(())
}
