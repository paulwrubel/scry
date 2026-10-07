mod color;
mod config;
mod error;
mod models;
mod service;
mod state;
mod store;
mod tui;

use crate::color::ColorChoice;
use crate::models::{
    Color, PROJECT_TEMPLATES, Priority, Project, ProjectTemplate, StatusStyle, Tags, Task,
    TaskSortingMode,
};
use crate::service::{ProjectService, TaskInput};
use crate::state::{DATETIME_FORMAT_STR, ProjectState};
use chrono::Local;
use clap::{Parser, Subcommand};
use config::ScryConfig;
use error::{AppError, ServiceError};
use store::{TaskStore, sqlite::SqliteStore};

#[derive(Parser)]
#[command(name = "scry", about = "A task manager for the terminal", version)]
struct Cli {
    /// Target a specific project (overrides the active project)
    #[arg(short = 'p', long = "project")]
    project: Option<String>,

    /// When to colorize output
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorChoice,

    /// Emit machine-readable JSON instead of human-formatted text
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task
    Add {
        /// The task title
        title: String,
        /// The task description
        #[arg(long)]
        description: Option<String>,
        /// The task priority
        #[arg(long)]
        priority: Option<Priority>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// The target status (defaults to the project entry status, otherwise the first status)
        #[arg(long)]
        status: Option<String>,
    },
    /// Move a task to a new status (alias for update --status)
    Move {
        /// The task ID
        id: i64,
        /// The target status
        status: String,
    },
    /// Update task properties
    Update {
        /// The task ID
        id: i64,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New description (empty string clears it)
        #[arg(long)]
        description: Option<String>,
        /// New priority
        #[arg(long)]
        priority: Option<Priority>,
        /// Comma-separated tags (empty string clears them)
        #[arg(long)]
        tags: Option<String>,
        /// Move the task to a new status
        #[arg(long)]
        status: Option<String>,
    },
    /// Duplicate an existing task
    Duplicate {
        /// The task ID
        id: i64,
    },
    /// Delete a task
    Delete {
        /// The task ID
        id: i64,
    },
    /// Show full details for a task
    Show {
        /// The task ID
        id: i64,
    },
    /// List tasks in the active project
    List {
        /// Show only tasks in a specific status
        #[arg(long)]
        status: Option<String>,
        /// Case-insensitive substring filter over task titles and tags
        #[arg(long)]
        search: Option<String>,
    },
    /// Manage notes on a task
    #[command(subcommand)]
    Note(NoteCommand),
    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommand),
}

#[derive(Subcommand)]
enum NoteCommand {
    /// Add a note to a task
    Add {
        /// The task ID
        task_id: i64,
        /// The note contents
        contents: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// List all projects
    List,
    /// Show the currently active project
    Current,
    /// Set the active project
    Use {
        /// The project name
        name: String,
    },
    /// Create a new project
    Create {
        /// The name of a template for the project
        #[arg(short = 't')]
        template_name: Option<String>,
        /// The project name
        name: String,
    },
    /// Rename a project
    Rename {
        /// The current project name
        old_name: String,
        /// The new project name
        new_name: String,
    },
    /// Set the entry status, so new tasks default to it
    SetEntryStatus {
        /// The status name
        name: String,
    },
    /// Reset the entry status, so new tasks default to the first status
    ResetEntryStatus,
    /// Set the task sorting mode for the project
    SetSort {
        /// The sorting mode
        mode: TaskSortingMode,
    },
    /// Show the priority level in the task list
    ShowPriority,
    /// Hide the priority level in the task list
    HidePriority,
    /// Delete a project and all its tasks
    Delete {
        /// The project name
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Manage statuses within a project
    #[command(subcommand)]
    Status(StatusCommand),
}

#[derive(Subcommand)]
enum StatusCommand {
    /// List statuses for a project
    List,
    /// Add a new status
    Add {
        /// The status name
        name: String,
    },
    /// Remove a status from a project. Requires the status have no assigned tasks.
    Remove {
        /// The status name
        name: String,
    },
    /// Rename a status
    Rename {
        /// The current status name
        old_name: String,
        /// The new status name
        new_name: String,
    },
    /// Move a status up in the ordering for the project
    MoveUp {
        /// The status name
        name: String,
    },
    /// Move a status down in the ordering for the project
    MoveDown {
        /// The status name
        name: String,
    },
    /// Set the style for a status
    SetStyle {
        /// The status name
        name: String,
        /// The style to apply
        style: StatusStyle,
    },
    /// Set the color for a status
    SetColor {
        /// The status name
        name: String,
        /// The color to apply
        color: Color,
    },
    /// Reset the color for a status
    ResetColor {
        /// The status name
        name: String,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let output_json = cli.json;

    match run(cli).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if output_json {
                eprintln!("{}", error_document(&error));
            } else {
                eprintln!("{error}");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

/// Render an error as the machine-readable document emitted on stderr under `--json`.
fn error_document(error: &AppError) -> String {
    serde_json::json!({
        "error": {
            "kind": error.kind(),
            "message": error.to_string(),
        }
    })
    .to_string()
}

/// Print a value as compact JSON on stdout, for the `--json` success path.
fn print_json<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string(value)
        .map_err(|error| AppError::Internal(format!("failed to serialize JSON: {error}")))?;
    println!("{json}");
    Ok(())
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let output_json = cli.json;
    if output_json && cli.command.is_none() {
        return Err(AppError::Usage(
            "--json is exclusive to cli commands, not the terminal UI".to_string(),
        ));
    }

    if output_json {
        ColorChoice::Never.apply();
    } else {
        cli.color.apply();
    }

    let config = ScryConfig::load()?;
    let store = SqliteStore::new(&config.database_url).await?;

    let project = resolve_project(&store, cli.project.as_deref()).await?;

    let Some(command) = cli.command else {
        let mut app = tui::App::new(config, store, project.id);
        return app.run().await;
    };

    let service = ProjectService::new(&store);

    match command {
        Command::Add {
            title,
            description,
            priority,
            tags,
            status,
        } => {
            let status_id = match status {
                Some(name) => Some(service.get_status_by_name(&project, &name).await?.id),
                None => None,
            };
            let new_task = TaskInput {
                title: Some(title),
                description,
                priority,
                tags: tags.map(|tags| Tags::from(tags.as_str())),
                status: status_id,
            };
            let change = service.create_task(&project, new_task).await?;
            if output_json {
                print_json(&change.task)?;
            } else {
                println!(
                    "Created task {} in \"{}\" [{}]: {}",
                    change.task.id, project.name, change.status.name, change.task.title
                );
            }
        }
        Command::Move { id, status } => {
            let status = service.get_status_by_name(&project, &status).await?;
            let change = service.move_task(&project, id, status.id).await?;
            if output_json {
                print_json(&change.task)?;
            } else {
                println!(
                    "Moved task {} --> \"{}\"",
                    change.task.id, change.status.name
                );
            }
        }
        Command::Update {
            id,
            title,
            description,
            priority,
            tags,
            status,
        } => {
            if title.is_none()
                && description.is_none()
                && priority.is_none()
                && tags.is_none()
                && status.is_none()
            {
                return Err(AppError::Usage(
                    "No flags provided. Use 'scry update --help' for available options."
                        .to_string(),
                ));
            }
            let status_id = match status {
                Some(name) => Some(service.get_status_by_name(&project, &name).await?.id),
                None => None,
            };
            let patch = TaskInput {
                title,
                description,
                priority,
                tags: tags.map(|tags| Tags::from(tags.as_str())),
                status: status_id,
            };
            let updated = service.update_task(&project, id, patch).await?;
            if output_json {
                print_json(&updated)?;
            } else {
                println!("Updated task {}.", updated.id);
            }
        }
        Command::Duplicate { id } => {
            let change = service.duplicate_task(&project, id).await?;
            if output_json {
                print_json(&change.task)?;
            } else {
                println!(
                    "Duplicated task {} as task {} in \"{}\" [{}]",
                    id, change.task.id, project.name, change.status.name
                );
            }
        }
        Command::Delete { id } => {
            let removed = service.delete_task(&project, id).await?;
            if output_json {
                print_json(&removed)?;
            } else {
                println!("Deleted task {} from \"{}\"", id, project.name);
            }
        }
        Command::Show { id } => {
            let state = ProjectState::load_from_store(&store, project.id).await?;

            let Some(task) = state.get_task_by_id(id) else {
                return Err(AppError::Service(ServiceError::TaskNotFound {
                    task_id: id,
                    project: project.name.clone(),
                }));
            };

            if output_json {
                print_json(task)?;
                return Ok(());
            }

            let status = state.get_status_by_id(task.status_id);
            let status_name = status.map(|s| s.name.as_str()).unwrap_or("unknown");
            let status_color = status.and_then(|s| s.color);
            let created_at = task
                .created_at
                .with_timezone(&Local)
                .format(DATETIME_FORMAT_STR);

            println!("{} #{}", task.title, task.id);
            println!();

            if let Some(description) = &task.description {
                for line in description.lines() {
                    println!("    {}", line);
                }
            }
            println!();

            anstream::println!("Priority:    {}", color::priority_long(task.priority));
            anstream::println!("Status:      {}", color::status(status_color, status_name));
            let tags = task
                .tags
                .iter()
                .map(|tag| color::tag(tag))
                .collect::<Vec<_>>()
                .join(" ");
            anstream::println!("Tags:        {}", tags);
            println!("Created at:  {}", created_at);
            println!();

            for note in &task.notes {
                let note_created_at = note
                    .created_at
                    .with_timezone(&Local)
                    .format(DATETIME_FORMAT_STR);
                println!("{}", note_created_at);
                println!("{}", note.contents);
                println!();
            }
        }
        Command::List { status, search } => {
            let mut state = ProjectState::load_from_store(&store, project.id).await?;
            if let Some(search) = search.filter(|s| !s.is_empty()) {
                state = state.with_substring_filter(search);
            }

            if output_json {
                let tasks: Vec<Task> = state
                    .statuses()
                    .filter(|status_def| match &status {
                        Some(filter) => status_def.name == *filter,
                        None => true,
                    })
                    .flat_map(|status_def| state.tasks_in_status(status_def.id))
                    .map(Task::from)
                    .collect();
                print_json(&tasks)?;
                return Ok(());
            }

            println!("project \"{}\"\n", state.project().name);

            let statuses_to_show: Vec<_> = state
                .statuses()
                .filter(|status_def| match &status {
                    Some(filter) => status_def.name == *filter,
                    None => true,
                })
                .collect();

            let total_tasks: usize = statuses_to_show
                .iter()
                .map(|status_def| state.tasks_in_status(status_def.id).len())
                .sum();

            if total_tasks == 0 {
                println!("No tasks.");
                return Ok(());
            }

            let show_priority = state.project().show_priority;
            let entry_status_id = state.project().entry_status_id;

            for status_def in statuses_to_show {
                let status_tasks = state.tasks_in_status(status_def.id);

                let marker = if entry_status_id == Some(status_def.id) {
                    "* "
                } else {
                    ""
                };
                let header = format!("{}{} ({}):", marker, status_def.name, status_tasks.len());
                anstream::println!("{}", color::status(status_def.color, &header));

                for task in &status_tasks {
                    let icon = match status_def.style {
                        StatusStyle::None | StatusStyle::Strikethrough => "",
                        StatusStyle::Unchecked => "[ ]",
                        StatusStyle::Checked => "[x]",
                    };

                    let mut line = format!("  {}", task.id);
                    if !icon.is_empty() {
                        line.push_str(&format!("  {}", icon));
                    }
                    if show_priority {
                        line.push_str(&format!("  {}", color::priority(task.priority)));
                    }
                    line.push_str(&format!("  {}", task.title));

                    let tags = task
                        .tags
                        .iter()
                        .map(|tag| color::tag(tag))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !tags.is_empty() {
                        line.push_str(&format!("  {}", tags));
                    }

                    anstream::println!("{}", line);
                }

                if !status_tasks.is_empty() {
                    println!();
                }
            }
        }
        Command::Note(note_cmd) => match note_cmd {
            NoteCommand::Add { task_id, contents } => {
                let note = service.add_task_note(&project, task_id, contents).await?;
                if output_json {
                    print_json(&note)?;
                } else {
                    println!("Added note {} to task {}.", note.id, note.task_id);
                }
            }
        },
        Command::Project(project_cmd) => match project_cmd {
            ProjectCommand::List => {
                let projects = store.get_all_projects().await?;
                if output_json {
                    print_json(&projects)?;
                } else if projects.is_empty() {
                    eprintln!("No projects. Run 'scry project create <name>' to create one.");
                } else {
                    for p in &projects {
                        let marker = if p.id == project.id { "* " } else { "  " };
                        println!("  {}{}", marker, p.name);
                    }
                }
            }
            ProjectCommand::Current => {
                if output_json {
                    print_json(&project)?;
                } else {
                    println!("{}", project.name);
                }
            }
            ProjectCommand::Use { name } => {
                let active = service.set_active_project(&name).await?;
                if output_json {
                    print_json(&active)?;
                } else {
                    println!("Using project \"{}\"", name);
                }
            }
            ProjectCommand::Create {
                name,
                template_name,
            } => {
                let template: Option<&ProjectTemplate> = match template_name {
                    Some(requested) => {
                        let Some(template) = PROJECT_TEMPLATES.iter().find(|t| t.name == requested)
                        else {
                            return Err(AppError::Usage(format!(
                                "Unknown template name: \"{}\".",
                                requested
                            )));
                        };
                        Some(template)
                    }
                    None => None,
                };

                let project = service.create_project(name, template.copied()).await?;

                if output_json {
                    print_json(&project)?;
                } else {
                    println!("Created project \"{}\"", project.name);
                    if template.is_none() {
                        println!(
                            "Note: this project has no statuses yet. Add one with 'scry project status add <name>'."
                        );
                    }
                    println!(
                        "Make it active with 'scry project use \"{}\"'.",
                        project.name
                    );
                }
            }
            ProjectCommand::Delete { name, force } => {
                if !force && !output_json {
                    use std::io::Write;
                    print!("Delete project \"{}\" and all its tasks? [y/N]: ", name);
                    std::io::stdout().flush().unwrap();
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();
                    if input.trim().to_lowercase() != "y" {
                        println!("Cancelled.");
                        return Ok(());
                    }
                }
                let removed = service.delete_project(&name).await?;
                if output_json {
                    print_json(&removed)?;
                } else {
                    println!("Deleted project \"{}\"", name);
                    let new_active = store.get_active_project().await?;
                    if new_active.name != name {
                        println!("Using project \"{}\"", new_active.name);
                    }
                }
            }
            ProjectCommand::Rename { old_name, new_name } => {
                let renamed = service.rename_project(&project, new_name.clone()).await?;
                if output_json {
                    print_json(&renamed)?;
                } else {
                    println!("Renamed project \"{}\" --> \"{}\"", old_name, new_name);
                }
            }
            ProjectCommand::SetEntryStatus { name } => {
                let status = service.get_status_by_name(&project, &name).await?;
                let updated = service
                    .set_project_entry_status(&project, Some(status.id))
                    .await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!(
                        "Set entry status of project \"{}\" to \"{}\"",
                        project.name, name
                    );
                }
            }
            ProjectCommand::ResetEntryStatus => {
                let updated = service.set_project_entry_status(&project, None).await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!("Reset entry status of project \"{}\"", project.name);
                }
            }
            ProjectCommand::SetSort { mode } => {
                let updated = service.set_project_sorting_mode(&project, mode).await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!(
                        "Set sort mode of project \"{}\" to \"{}\"",
                        project.name, mode
                    );
                }
            }
            ProjectCommand::ShowPriority => {
                let updated = service
                    .set_project_should_show_priority(&project, true)
                    .await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!("Showing priority in project \"{}\"", project.name);
                }
            }
            ProjectCommand::HidePriority => {
                let updated = service
                    .set_project_should_show_priority(&project, false)
                    .await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!("Hiding priority in project \"{}\"", project.name);
                }
            }
            ProjectCommand::Status(status_cmd) => match status_cmd {
                StatusCommand::List => {
                    let statuses = store.get_all_statuses_by_project_id(project.id).await?;
                    if output_json {
                        print_json(&statuses)?;
                    } else {
                        println!("Statuses for \"{}\":", project.name);
                        for s in &statuses {
                            anstream::println!("  {}", color::status(s.color, &s.name));
                        }
                    }
                }
                StatusCommand::Add { name } => {
                    let status = service.create_status(&project, name).await?;
                    if output_json {
                        print_json(&status)?;
                    } else {
                        println!(
                            "Added status \"{}\" to project \"{}\"",
                            status.name, project.name
                        );
                    }
                }
                StatusCommand::Remove { name } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let removed = service.delete_status(&project, status.id).await?;
                    if output_json {
                        print_json(&removed)?;
                    } else {
                        println!(
                            "Removed status \"{}\" from project \"{}\"",
                            name, project.name
                        );
                    }
                }
                StatusCommand::Rename { old_name, new_name } => {
                    let status = service.get_status_by_name(&project, &old_name).await?;
                    let renamed = service
                        .rename_status(&project, status.id, new_name.clone())
                        .await?;
                    if output_json {
                        print_json(&renamed)?;
                    } else {
                        println!(
                            "Renamed status \"{}\" --> \"{}\" in project \"{}\"",
                            old_name, new_name, project.name
                        );
                    }
                }
                StatusCommand::MoveUp { name } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let moved = service.move_status_up(&project, status.id).await?;
                    if output_json {
                        print_json(&status)?;
                    } else {
                        match moved {
                            Some(_) => println!(
                                "Moved status \"{}\" up in project \"{}\"",
                                name, project.name
                            ),
                            None => eprintln!(
                                "Status \"{}\" is already at the top of \"{}\"",
                                name, project.name
                            ),
                        }
                    }
                }
                StatusCommand::MoveDown { name } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let moved = service.move_status_down(&project, status.id).await?;
                    if output_json {
                        print_json(&status)?;
                    } else {
                        match moved {
                            Some(_) => println!(
                                "Moved status \"{}\" down in project \"{}\"",
                                name, project.name
                            ),
                            None => eprintln!(
                                "Status \"{}\" is already at the bottom of \"{}\"",
                                name, project.name
                            ),
                        }
                    }
                }
                StatusCommand::SetStyle { name, style } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let updated = service.set_status_style(&project, status.id, style).await?;
                    if output_json {
                        print_json(&updated)?;
                    } else {
                        println!(
                            "Set style of status \"{}\" to \"{}\" in project \"{}\"",
                            name, style, project.name
                        );
                    }
                }
                StatusCommand::SetColor { name, color } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let updated = service
                        .set_status_color(&project, status.id, Some(color))
                        .await?;
                    if output_json {
                        print_json(&updated)?;
                    } else {
                        println!(
                            "Set color of status \"{}\" to \"{}\" in project \"{}\"",
                            name, color, project.name
                        );
                    }
                }
                StatusCommand::ResetColor { name } => {
                    let status = service.get_status_by_name(&project, &name).await?;
                    let updated = service.set_status_color(&project, status.id, None).await?;
                    if output_json {
                        print_json(&updated)?;
                    } else {
                        println!(
                            "Reset color of status \"{}\" in project \"{}\"",
                            name, project.name
                        );
                    }
                }
            },
        },
    }

    Ok(())
}

/// Resolve which project to use: --project flag takes precedence over active project.
async fn resolve_project(store: &SqliteStore, flag: Option<&str>) -> Result<Project, AppError> {
    if let Some(name) = flag {
        let project = store
            .get_project_by_name(name)
            .await?
            .ok_or_else(|| AppError::Usage(format!("project '{}' not found", name)))?;
        Ok(project)
    } else {
        let project = store.get_active_project().await?;
        Ok(project)
    }
}
