//! `section move`.

use scry_core::models::{Project, Section, SectionId};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Where to move a section relative to its siblings.
pub enum SectionMovePlacement<'a> {
    Top,
    Bottom,
    Before(&'a str),
    After(&'a str),
}

/// Move a section to the top, bottom, or next to another section.
pub async fn section_move(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    name: &str,
    placement: SectionMovePlacement<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let section = section_by_name(client, &project, name).await?;

    #[derive(Serialize)]
    struct MoveSectionRequest {
        placement: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        anchor_id: Option<SectionId>,
    }

    let (placement_kind, anchor_id): (&'static str, Option<SectionId>) = match &placement {
        SectionMovePlacement::Top => ("top", None),
        SectionMovePlacement::Bottom => ("bottom", None),
        SectionMovePlacement::Before(anchor_name) => {
            let anchor = section_by_name(client, &project, anchor_name).await?;
            ("before", Some(anchor.id))
        }
        SectionMovePlacement::After(anchor_name) => {
            let anchor = section_by_name(client, &project, anchor_name).await?;
            ("after", Some(anchor.id))
        }
    };

    let section_id = section.id.to_string();
    let updated: Section = client
        .post_json(
            &["sections", section_id.as_str(), "move"],
            &[],
            &MoveSectionRequest {
                placement: placement_kind,
                anchor_id,
            },
        )
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        match &placement {
            SectionMovePlacement::Top => println!(
                "Moved section \"{}\" to the top of project \"{}\"",
                name, project.name
            ),
            SectionMovePlacement::Bottom => println!(
                "Moved section \"{}\" to the bottom of project \"{}\"",
                name, project.name
            ),
            SectionMovePlacement::Before(anchor_name) => println!(
                "Moved section \"{}\" before \"{}\" in project \"{}\"",
                name, anchor_name, project.name
            ),
            SectionMovePlacement::After(anchor_name) => println!(
                "Moved section \"{}\" after \"{}\" in project \"{}\"",
                name, anchor_name, project.name
            ),
        }
    }

    Ok(())
}
