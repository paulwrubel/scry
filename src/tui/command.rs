use clap::{Parser, Subcommand};

use crate::tui::Action;
use crate::tui::component::popup::ConfirmDeleteEntity;
use crate::tui::component::{ProjectStatusTasks, State};

/// The TUI commands
#[derive(Parser)]
#[command(
    name = "scry",
    // no overarching binary token to start, we just use the subcommand directly
    multicall = true,
    // no help functionality in the in-TUI command input
    disable_help_flag = true,
    disable_help_subcommand = true
)]
enum Command {
    #[command(subcommand)]
    Status(StatusCommand),
}

#[derive(Subcommand)]
enum StatusCommand {
    /// Add a Status to this project
    Add {
        /// Name of the Status to add
        name: String,
    },
    /// Felete a Status from this project
    Delete {
        /// Name of the Status to delete
        name: String,
    },
    MoveUp {
        /// Name of the Status to move up
        name: String,
    },
    MoveDown {
        /// Name of the Status to move down
        name: String,
    },
}

/// Parse a command line (without the leading "/") into an Action to emit.
pub fn parse_command(state: &State, line: &str) -> Vec<Action> {
    let Some(tokens) = shlex::split(line).filter(|t| !t.is_empty()) else {
        return vec![];
    };

    let command = match Command::try_parse_from(tokens) {
        Ok(command) => command,
        Err(err) => return vec![Action::OpenPopupErrorInfo(err.to_string())],
    };

    let project_status_tasks = ProjectStatusTasks::from(state);
    match command {
        Command::Status(status_command) => match status_command {
            StatusCommand::Add { name } => vec![Action::AddStatus(name)],
            StatusCommand::Delete { name } => {
                if let Some(status) = state.statuses.iter().find(|s| s.name == name) {
                    let status_tasks = project_status_tasks.tasks_in_status(status.id);
                    if status_tasks.is_empty() {
                        vec![Action::OpenPopupConfirmDelete(ConfirmDeleteEntity::Status(
                            status.clone(),
                        ))]
                    } else {
                        vec![Action::OpenPopupErrorInfo(format![
                            "status \"{}\" has {} tasks assigned. Please delete or move tasks in \"{}\" before deleting.",
                            name,
                            status_tasks.len(),
                            name
                        ])]
                    }
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::MoveUp { name } => {
                if let Some(status) = state.statuses.iter().find(|s| s.name == name) {
                    vec![Action::ReorderStatus {
                        id: status.id,
                        new_position: status.position.saturating_sub(1),
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::MoveDown { name } => {
                if let Some(status) = state.statuses.iter().find(|s| s.name == name) {
                    vec![Action::ReorderStatus {
                        id: status.id,
                        new_position: status.position.saturating_add(1),
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
        },
    }
}
