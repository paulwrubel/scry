//! Client command implementations.

mod add;
pub use add::{AddTask, add};

mod close_tasks;
pub use close_tasks::close_tasks;

mod delete;
pub use delete::delete;

mod duplicate;
pub use duplicate::duplicate;

mod list;
pub use list::list;

mod move_task;
pub use move_task::{MoveTask, move_task};

mod note_add;
pub use note_add::note_add;

mod open_tasks;
pub use open_tasks::open_tasks;

mod project_create;
pub use project_create::{CreateProject, project_create};

mod project_current;
pub use project_current::project_current;

mod project_delete;
pub use project_delete::project_delete;

mod project_list;
pub use project_list::project_list;

mod project_show;
pub use project_show::project_show;

mod project_update;
pub use project_update::{UpdateProject, project_update};

mod project_use;
pub use project_use::project_use;

mod section_add;
pub use section_add::section_add;

mod section_list;
pub use section_list::section_list;

mod section_merge;
pub use section_merge::section_merge;

mod section_remove;
pub use section_remove::section_remove;

mod section_rename;
pub use section_rename::section_rename;

mod section_reset_color;
pub use section_reset_color::section_reset_color;

mod section_set_color;
pub use section_set_color::section_set_color;

mod show;
pub use show::show;

mod update;
pub use update::{UpdateTask, update};

use scry_core::models::{Project, Section};

use crate::client::{BatchTaskResult, Client};
use crate::client_state::ClientState;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

/// Resolve a project from `-p` or the local active project.
///
/// An explicit `-p` must exist. A stale active project falls back to `default`,
/// matching the old local resolver.
async fn project_by_name(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
) -> Result<Project, AppError> {
    if let Some(name) = project_flag {
        return fetch_project(client, name).await;
    }

    if let Some(name) = ClientState::load(config)?.active_project
        && let Some(project) = fetch_project_opt(client, &name).await?
    {
        return Ok(project);
    }

    fetch_project(client, "default").await
}

async fn fetch_project_opt(client: &Client, name: &str) -> Result<Option<Project>, AppError> {
    client.get_json(&["projects", "by-name", name], &[]).await
}

async fn fetch_project(client: &Client, name: &str) -> Result<Project, AppError> {
    fetch_project_opt(client, name)
        .await?
        .ok_or_else(|| AppError::Remote {
            kind: "not_found",
            message: format!("project '{name}' not found"),
            details: None,
        })
}

/// Resolve a section by exact name within a project.
async fn section_by_name(
    client: &Client,
    project: &Project,
    name: &str,
) -> Result<Section, AppError> {
    let project_id = project.id.to_string();
    client
        .get_json(
            &["projects", project_id.as_str(), "sections", "by-name", name],
            &[],
        )
        .await
}

/// Render the per-handle outcomes of a batch lifecycle command.
fn render_batch_results(
    results: &[BatchTaskResult],
    past: &str,
    verb: &str,
    output_json: bool,
) -> Result<(), AppError> {
    if output_json {
        print_json(results)?;
        return Ok(());
    }

    let mut failures = 0;
    for result in results {
        if let Some(error) = &result.error {
            failures += 1;
            eprintln!(
                "Failed to {} task {}: {}",
                verb, result.handle, error.message
            );
        } else {
            println!("{} task {}.", past, result.handle);
        }
    }

    if failures > 0 {
        return Err(AppError::Usage(format!(
            "{} task(s) could not be updated",
            failures
        )));
    }

    Ok(())
}
