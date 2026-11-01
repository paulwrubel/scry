//! `section merge`.

use scry_core::models::{Project, Section};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::{project_by_name, section_by_name};

/// Merge one section into another.
pub async fn section_merge(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    source_name: &str,
    target_name: &str,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let source = section_by_name(client, &project, source_name).await?;
    let target = section_by_name(client, &project, target_name).await?;

    #[derive(Serialize)]
    struct MergeSectionRequest {
        target_id: scry_core::ids::SectionId,
    }

    let source_id = source.id.to_string();
    let merged: Section = client
        .post_json(
            &["sections", source_id.as_str(), "merge"],
            &[],
            &MergeSectionRequest {
                target_id: target.id,
            },
        )
        .await?;

    if output_json {
        print_json(&merged)?;
    } else {
        println!(
            "Merged section \"{}\" into \"{}\"",
            source_name, merged.name
        );
    }

    Ok(())
}
