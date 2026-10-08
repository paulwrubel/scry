use clap::{Parser, Subcommand};

use scry_core::models::{Color, StatusStyle, TaskSortingMode};
use crate::state::ProjectState;
use crate::tui::Action;
use crate::tui::component::popup::ConfirmDeleteEntity;

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
    #[command(subcommand, alias("s"))]
    Status(StatusCommand),

    #[command(subcommand, alias("p"))]
    Project(ProjectCommand),
}

#[derive(Subcommand)]
enum StatusCommand {
    /// Add a Status to this project
    #[command(aliases(["a"]))]
    Add {
        /// Name of the Status to add
        name: String,
    },
    /// Delete a Status from this project
    #[command(aliases(["del"]))]
    Delete {
        /// Name of the Status to delete
        name: String,
    },
    /// Rename a Status in this project
    #[command(aliases(["r"]))]
    Rename {
        /// Current name of the status
        old: String,
        /// New name of the status
        new: String,
    },
    /// Move a Status up in the ordering for the project
    #[command(aliases(["up", "mu"]))]
    MoveUp {
        /// Name of the Status to move up
        name: String,
    },
    /// Move a Status down in the ordering for the project
    #[command(aliases(["down", "md"]))]
    MoveDown {
        /// Name of the Status to move down
        name: String,
    },
    /// Set the color for a Status in this project
    #[command(aliases(["color", "sc"]))]
    SetColor {
        /// Name of the Status to set the color for
        name: String,
        /// Color for the Status
        color: Color,
    },
    /// Reset the color for a Status in this project
    #[command(alias("rs"))]
    ResetColor {
        /// Name of the Status to reset the color for
        name: String,
    },
    /// Set the style for a Status in this project
    #[command(alias("ss"))]
    SetStyle {
        /// Name of the Status to set the style for
        name: String,
        /// Style for the Status
        style: StatusStyle,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Set a Status as the "entry" Status, meaning new tasks will default to this status
    #[command(aliases(["entry-status", "ses"]))]
    SetEntryStatus {
        /// Name of the Status to add
        status_name: String,
    },
    /// Reset a Status as the "entry" Status, meaning new tasks will default to the first status instead
    #[command(alias("res"))]
    ResetEntryStatus,
    /// Set the sorting mode for tasks in this project.
    ///
    /// The default mode is alphabetical
    #[command(aliases(["setsort", "set-sort", "sort", "ss", "s"]))]
    SetTaskSortingMode {
        /// The sorting mode to use for tasks
        task_sorting_mode: TaskSortingMode,
    },
    /// Show the priority level in the task title line
    #[command(aliases(["sp"]))]
    ShowPriority,
    /// Hide the priority level in the task title line
    #[command(aliases(["hp"]))]
    HidePriority,
}

/// Parse a command line (without the leading "/") into an Action to emit.
pub fn parse_command(state: &ProjectState, line: &str) -> Vec<Action> {
    let Some(tokens) = shlex::split(line).filter(|t| !t.is_empty()) else {
        return vec![];
    };

    let command = match Command::try_parse_from(tokens) {
        Ok(command) => command,
        Err(err) => return vec![Action::OpenPopupErrorInfo(err.to_string())],
    };

    match command {
        Command::Status(status_command) => match status_command {
            StatusCommand::Add { name } => vec![Action::CreateStatus { name }],
            StatusCommand::Delete { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::OpenPopupConfirmDelete(ConfirmDeleteEntity::Status(
                        status.clone(),
                    ))]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::Rename { old, new } => {
                if let Some(status) = state.get_status_by_name(&old) {
                    vec![Action::RenameStatus {
                        status_id: status.id,
                        new_name: new,
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        old
                    ])]
                }
            }
            StatusCommand::MoveUp { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::MoveStatusUp {
                        status_id: status.id,
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::MoveDown { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::MoveStatusDown {
                        status_id: status.id,
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::SetColor { name, color } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::SetStatusColor {
                        status_id: status.id,
                        color: Some(color),
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::ResetColor { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::SetStatusColor {
                        status_id: status.id,
                        color: None,
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::SetStyle { name, style } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::SetStatusStyle {
                        status_id: status.id,
                        style,
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
        },
        Command::Project(project_command) => match project_command {
            ProjectCommand::SetEntryStatus { status_name } => {
                if let Some(status) = state.get_status_by_name(&status_name) {
                    vec![Action::SetProjectEntryStatus {
                        status_id: Some(status.id),
                    }]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        status_name
                    ])]
                }
            }
            ProjectCommand::ResetEntryStatus => {
                vec![Action::SetProjectEntryStatus { status_id: None }]
            }
            ProjectCommand::SetTaskSortingMode { task_sorting_mode } => {
                vec![Action::SetProjectSortingMode(task_sorting_mode)]
            }
            ProjectCommand::ShowPriority => {
                if !state.project().show_priority {
                    vec![Action::SetProjectShouldShowPriority(true)]
                } else {
                    vec![]
                }
            }
            ProjectCommand::HidePriority => {
                if state.project().show_priority {
                    vec![Action::SetProjectShouldShowPriority(false)]
                } else {
                    vec![]
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use clap::Parser;

    #[test]
    fn status_set_style_hidden_command_parses() {
        // parsing builds the entire clap command tree, so this guards against
        // duplicate-alias debug assertions firing before any subcommand runs
        Command::try_parse_from(["status", "set-style", "super done", "hidden"])
            .expect("status set-style with the hidden style should parse");
    }
}
