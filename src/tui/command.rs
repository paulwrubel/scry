use clap::{Parser, Subcommand};

use crate::models::{Color, Project, Status, StatusStyle, TaskSortingMode};
use crate::tui::Action;
use crate::tui::component::popup::ConfirmDeleteEntity;
use crate::tui::state::ProjectState;

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
    #[command(aliases(["d"]))]
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
    #[command(aliases(["up", "mu", "u"]))]
    MoveUp {
        /// Name of the Status to move up
        name: String,
    },
    /// Move a Status down in the ordering for the project
    #[command(aliases(["down", "md", "d"]))]
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
            StatusCommand::Add { name } => {
                let position = state.statuses().map(|s| s.position).max().unwrap_or(0) + 1;
                vec![Action::CreateStatus(Status {
                    id: 0,
                    project_id: state.project().id,
                    name,
                    position,
                    color: None,
                    style: StatusStyle::None,
                })]
            }
            StatusCommand::Delete { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    let status_tasks = state.tasks_in_status(status.id);
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
            StatusCommand::Rename { old, new } => {
                if let Some(status) = state.get_status_by_name(&old) {
                    if state.statuses().find(|s| s.name == new).is_none() {
                        vec![Action::UpdateStatus(Status {
                            id: status.id,
                            name: new,
                            ..status.clone()
                        })]
                    } else {
                        vec![Action::OpenPopupErrorInfo(format![
                            "status with name \"{}\" already exists in project",
                            new
                        ])]
                    }
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        old
                    ])]
                }
            }
            StatusCommand::MoveUp { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    if status.position == 0 {
                        vec![]
                    } else {
                        vec![Action::UpdateStatus(Status {
                            id: status.id,
                            position: status.position.saturating_sub(1),
                            ..status.clone()
                        })]
                    }
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::MoveDown { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    let max_position = state.statuses().map(|s| s.position).max().unwrap_or(0);
                    if status.position >= max_position {
                        vec![]
                    } else {
                        vec![Action::UpdateStatus(Status {
                            id: status.id,
                            position: status.position.saturating_add(1),
                            ..status.clone()
                        })]
                    }
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::SetColor { name, color } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::UpdateStatus(Status {
                        id: status.id,
                        color: Some(color),
                        ..status.clone()
                    })]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::ResetColor { name } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::UpdateStatus(Status {
                        id: status.id,
                        color: None,
                        ..status.clone()
                    })]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        name
                    ])]
                }
            }
            StatusCommand::SetStyle { name, style } => {
                if let Some(status) = state.get_status_by_name(&name) {
                    vec![Action::UpdateStatus(Status {
                        id: status.id,
                        style,
                        ..status.clone()
                    })]
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
                    vec![Action::UpdateProject(Project {
                        entry_status_id: Some(status.id),
                        ..state.project().clone()
                    })]
                } else {
                    vec![Action::OpenPopupErrorInfo(format![
                        "no status with name \"{}\" found in project",
                        status_name
                    ])]
                }
            }
            ProjectCommand::ResetEntryStatus => {
                vec![Action::UpdateProject(Project {
                    entry_status_id: None,
                    ..state.project().clone()
                })]
            }
            ProjectCommand::SetTaskSortingMode { task_sorting_mode } => {
                vec![Action::UpdateProject(Project {
                    task_sorting_mode,
                    ..state.project().clone()
                })]
            }
        },
    }
}
