//! `section reset-color`.

use scry_core::models::{Color, Project, Section};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Reset a section's color.
pub async fn section_reset_color(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    name: &str,
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
                color: None,
            },
        )
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        println!(
            "Reset color of section \"{}\" in project \"{}\"",
            name, project.name
        );
    }

    Ok(())
}
