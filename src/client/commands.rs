//! Client command implementations.

mod add;
pub use add::{AddTask, add};

mod list;
pub use list::list;

mod move_task;
pub use move_task::{MoveTask, move_task};

mod project_list;
pub use project_list::project_list;

mod show;
pub use show::show;

mod update;
pub use update::{UpdateTask, update};

use crate::client::Client;
use crate::client_state::ClientState;
use crate::config::ScryConfig;
use crate::error::AppError;
use scry_core::models::Project;

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
