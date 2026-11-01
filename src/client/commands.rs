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

mod project_list;
pub use project_list::project_list;

mod show;
pub use show::show;

mod update;
pub use update::{UpdateTask, update};

use scry_core::models::Project;

use crate::client::{BatchTaskResult, Client};
use crate::client_state::ClientState;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::print_json;

/// Resolve a project from `-p` or the local active project.
async fn project_by_name(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
) -> Result<Project, AppError> {
    let project_name = match project_flag {
        Some(name) => name.to_string(),
        None => ClientState::load(config)?
            .active_project
            .unwrap_or_else(|| "default".to_string()),
    };

    let project: Option<Project> = client
        .get_json(&["projects", "by-name", project_name.as_str()], &[])
        .await?;

    project.ok_or_else(|| AppError::Remote {
        kind: "not_found",
        message: format!("project '{project_name}' not found"),
        details: None,
    })
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
