//! `add`.

use scry_core::models::{Priority, Section, SectionId};
use serde::Serialize;

use crate::client::{Client, TaskDetails};
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// Input for the `add` command.
pub struct AddTask<'a> {
    pub project: Option<&'a str>,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub priority: Option<Priority>,
    pub tags: Option<&'a str>,
    pub section: Option<&'a str>,
    pub new_section: Option<&'a str>,
}

/// Add a task to a project.
pub async fn add(
    client: &Client,
    config: &ScryConfig,
    input: AddTask<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    let project = project_by_name(client, config, input.project).await?;

    let (section_id, new_section_name) = match (input.section, input.new_section) {
        (Some(_), Some(_)) => {
            return Err(AppError::Usage(
                "specify at most one of --section or --new-section".to_string(),
            ));
        }
        (Some(name), None) => {
            let project_id = project.id.to_string();
            let section: Section = client
                .get_json(
                    &["projects", project_id.as_str(), "sections", "by-name", name],
                    &[],
                )
                .await?;
            (Some(section.id), None)
        }
        (None, Some(name)) => (None, Some(name)),
        (None, None) => (None, None),
    };

    let tags: Option<Vec<String>> = input.tags.map(|tags| {
        tags.split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect()
    });

    #[derive(Serialize)]
    struct CreateTaskRequest<'a> {
        title: &'a str,
        description: Option<&'a str>,
        priority: Option<Priority>,
        tags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        section_id: Option<SectionId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        new_section: Option<&'a str>,
    }

    let body = CreateTaskRequest {
        title: input.title,
        description: input.description,
        priority: input.priority,
        tags,
        section_id,
        new_section: new_section_name,
    };

    let project_id = project.id.to_string();
    let detail: TaskDetails = client
        .post_json(&["projects", project_id.as_str(), "tasks"], &[], &body)
        .await?;

    if output_json {
        print_json(&detail)?;
    } else {
        let handle = detail.handle.as_deref().unwrap_or("?");
        let section = input.section.or(input.new_section).unwrap_or("No section");
        println!(
            "Created task {} in \"{}\" [{}]: {}",
            handle, project.name, section, detail.task.title
        );
    }

    Ok(())
}
