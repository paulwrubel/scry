use crate::service::{EntryStatusUpdate, ProjectUpdate};
use crate::state::ProjectState;
use crate::tui::Action;
use crate::tui::component::popup::ConfirmDeleteEntity;
use clap::{Parser, Subcommand};
use scry_core::models::{Color, StatusStyle, TaskSortingMode};

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
    /// Update project properties
    #[command(aliases(["up", "set"]))]
    Update {
        /// New project name
        #[arg(long)]
        name: Option<String>,
        /// New project description (empty string clears it)
        #[arg(long)]
        description: Option<String>,
        /// Set the entry status new tasks default to
        #[arg(long, conflicts_with = "clear_entry_status")]
        entry_status: Option<String>,
        /// Clear the entry status so new tasks use the first status
        #[arg(long, conflicts_with = "entry_status")]
        clear_entry_status: bool,
        /// Set the task sorting mode
        #[arg(long)]
        sort: Option<TaskSortingMode>,
        /// Show the priority level in task title lines
        #[arg(long, conflicts_with = "hide_priority")]
        show_priority: bool,
        /// Hide the priority level in task title lines
        #[arg(long, conflicts_with = "show_priority")]
        hide_priority: bool,
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
            ProjectCommand::Update {
                name,
                description,
                entry_status,
                clear_entry_status,
                sort,
                show_priority,
                hide_priority,
            } => {
                if name.is_none()
                    && description.is_none()
                    && entry_status.is_none()
                    && sort.is_none()
                    && !clear_entry_status
                    && !show_priority
                    && !hide_priority
                {
                    return vec![Action::OpenPopupErrorInfo(
                        "No update flags provided".to_string(),
                    )];
                }

                let entry_status = if clear_entry_status {
                    EntryStatusUpdate::Clear
                } else if let Some(status_name) = entry_status {
                    match state.get_status_by_name(&status_name) {
                        Some(status) => EntryStatusUpdate::Set(status.id),
                        None => {
                            return vec![Action::OpenPopupErrorInfo(format![
                                "no status with name \"{}\" found in project",
                                status_name
                            ])];
                        }
                    }
                } else {
                    EntryStatusUpdate::Unchanged
                };

                let show_priority = if show_priority {
                    Some(true)
                } else if hide_priority {
                    Some(false)
                } else {
                    None
                };

                vec![Action::UpdateProject(ProjectUpdate {
                    name,
                    description,
                    entry_status,
                    task_sorting_mode: sort,
                    show_priority,
                })]
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, ProjectCommand};
    use clap::Parser;
    use scry_core::models::TaskSortingMode;

    #[test]
    fn status_set_style_hidden_command_parses() {
        // parsing builds the entire clap command tree, so this guards against
        // duplicate-alias debug assertions firing before any subcommand runs
        Command::try_parse_from(["status", "set-style", "super done", "hidden"])
            .expect("status set-style with the hidden style should parse");
    }

    #[test]
    fn project_update_parses_all_flags() {
        let command = Command::try_parse_from([
            "project",
            "update",
            "--name",
            "beta",
            "--description",
            "notes",
            "--entry-status",
            "done",
            "--sort",
            "priority",
            "--show-priority",
        ])
        .expect("project update with every flag should parse");

        let Command::Project(ProjectCommand::Update {
            name,
            description,
            entry_status,
            clear_entry_status,
            sort,
            show_priority,
            hide_priority,
        }) = command
        else {
            panic!("expected the project update command");
        };

        assert_eq!(name.as_deref(), Some("beta"));
        assert_eq!(description.as_deref(), Some("notes"));
        assert_eq!(entry_status.as_deref(), Some("done"));
        assert!(!clear_entry_status);
        assert!(matches!(sort, Some(TaskSortingMode::Priority)));
        assert!(show_priority);
        assert!(!hide_priority);
    }

    #[test]
    fn project_update_rejects_conflicting_flags() {
        assert!(
            Command::try_parse_from([
                "project",
                "update",
                "--entry-status",
                "done",
                "--clear-entry-status",
            ])
            .is_err(),
            "entry status and clear entry status conflict"
        );

        assert!(
            Command::try_parse_from(["project", "update", "--show-priority", "--hide-priority"])
                .is_err(),
            "show priority and hide priority conflict"
        );
    }
}
