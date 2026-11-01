//! `section rename`.

use scry_core::models::{Color, Project, Section};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Rename a section.
pub async fn section_rename(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    old_name: &str,
    new_name: &str,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let section = section_by_name(client, &project, old_name).await?;

    #[derive(Serialize)]
    struct UpdateSectionRequest<'a> {
        name: &'a str,
        color: Option<Color>,
    }

    let section_id = section.id.to_string();
    let renamed: Section = client
        .put_json(
            &["sections", section_id.as_str()],
            &[],
            &UpdateSectionRequest {
                name: new_name,
                color: section.color,
            },
        )
        .await?;

    if output_json {
        print_json(&renamed)?;
    } else {
        println!(
            "Renamed section \"{}\" --> \"{}\" in project \"{}\"",
            old_name, new_name, project.name
        );
    }

    Ok(())
}
