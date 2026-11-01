//! `update`.

use scry_core::models::{Priority, Section, SectionId, Task};
use serde::Serialize;

use crate::client::{Client, TaskDetails};
use crate::error::AppError;
use crate::output::print_json;

/// Input for the `update` command.
pub struct UpdateTask<'a> {
    pub handle: &'a str,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub priority: Option<Priority>,
    pub tags: Option<&'a str>,
    pub section: Option<&'a str>,
    pub new_section: Option<&'a str>,
    pub no_section: bool,
}

/// Update a task.
pub async fn update(
    client: &Client,
    input: UpdateTask<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    if input.title.is_none()
        && input.description.is_none()
        && input.priority.is_none()
        && input.tags.is_none()
        && input.section.is_none()
        && input.new_section.is_none()
        && !input.no_section
    {
        return Err(AppError::Usage(
            "No flags provided. Use 'scry update --help' for available options.".to_string(),
        ));
    }

    let detail: TaskDetails = client
        .get_json(&["tasks", input.handle], &[("include_handle", "true")])
        .await?;

    let project_id = detail.task.project_id.to_string();

    let section_id = match input.section {
        Some(name) => {
            let section: Section = client
                .get_json(
                    &["projects", project_id.as_str(), "sections", "by-name", name],
                    &[],
                )
                .await?;
            Some(section.id)
        }
        None => None,
    };

    let tags: Option<Vec<String>> = input.tags.map(|tags| {
        tags.split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect()
    });

    #[derive(Serialize)]
    struct UpdateTaskRequest<'a> {
        title: Option<&'a str>,
        description: Option<&'a str>,
        priority: Option<Priority>,
        tags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        section_id: Option<SectionId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        new_section: Option<&'a str>,
        clear_section: bool,
    }

    let body = UpdateTaskRequest {
        title: input.title,
        description: input.description,
        priority: input.priority,
        tags,
        section_id,
        new_section: input.new_section,
        clear_section: input.no_section,
    };

    let task: Task = client
        .put_json(&["tasks", input.handle], &[], &body)
        .await?;

    if output_json {
        print_json(&task)?;
    } else {
        let handle = detail.handle.as_deref().unwrap_or(input.handle);
        println!("Updated task {}.", handle);
    }

    Ok(())
}
