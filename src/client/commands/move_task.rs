//! `move`.

use scry_core::models::{Section, SectionId, Task};
use serde::Serialize;

use crate::client::{Client, TaskDetails};
use crate::error::AppError;
use crate::output::print_json;

/// Input for the `move` command.
pub struct MoveTask<'a> {
    pub handle: &'a str,
    pub section: Option<&'a str>,
    pub no_section: bool,
}

/// Move a task to a section, or to No section.
pub async fn move_task(
    client: &Client,
    input: MoveTask<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    if input.section.is_none() && !input.no_section {
        return Err(AppError::Usage(
            "Specify a section or --no-section.".to_string(),
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

    #[derive(Serialize)]
    struct MoveTaskRequest {
        section_id: Option<SectionId>,
    }

    let task: Task = client
        .post_json(
            &["tasks", input.handle, "move"],
            &[],
            &MoveTaskRequest { section_id },
        )
        .await?;

    if output_json {
        print_json(&task)?;
    } else {
        let handle = detail.handle.as_deref().unwrap_or(input.handle);
        let section = input.section.unwrap_or("No section");
        println!("Moved task {} --> \"{}\"", handle, section);
    }

    Ok(())
}
