//! `project update`.

use scry_core::models::{Project, TaskSortingMode};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

use super::project_by_name;

/// Input for `project update`.
pub struct UpdateProject<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub sort: Option<TaskSortingMode>,
    pub show_priority: Option<bool>,
}

/// Update a project.
pub async fn project_update(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    input: UpdateProject<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    if input.name.is_none()
        && input.description.is_none()
        && input.sort.is_none()
        && input.show_priority.is_none()
    {
        return Err(AppError::Usage(
            "No flags provided. Use 'scry project update --help' for available options."
                .to_string(),
        ));
    }

    let project = project_by_name(client, config, project_flag).await?;

    #[derive(Serialize)]
    struct UpdateProjectRequest<'a> {
        name: &'a str,
        description: Option<&'a str>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    }

    let body = UpdateProjectRequest {
        name: input.name.unwrap_or(project.name.as_str()),
        description: input.description.or(project.description.as_deref()),
        task_sorting_mode: input.sort.unwrap_or(project.task_sorting_mode),
        show_priority: input.show_priority.unwrap_or(project.show_priority),
    };

    let project_id = project.id.to_string();
    let updated: Project = client
        .put_json(&["projects", project_id.as_str()], &[], &body)
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        println!("Updated project \"{}\".", updated.name);
    }

    Ok(())
}
