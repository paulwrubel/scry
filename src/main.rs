mod config;
mod error;
mod models;
mod state;
mod store;
mod tui;

use crate::models::{
    PROJECT_TEMPLATES, Priority, Project, ProjectTemplate, Status, StatusStyle, Tags, Task,
    TaskSortingMode,
};
use crate::state::{DATETIME_FORMAT_STR, ProjectState};
use crate::store::TaskToCreate;
use chrono::Local;
use clap::{Parser, Subcommand};
use config::ScryConfig;
use error::AppError;
use store::{TaskStore, sqlite::SqliteStore};

#[derive(Parser)]
#[command(name = "scry", about = "A task manager for the terminal", version)]
struct Cli {
    /// Target a specific project (overrides the active project)
    #[arg(short = 'p', long = "project")]
    project: Option<String>,

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
    /// Set the style for a status
    SetStyle {
        /// The status name
        name: String,
        /// The style to apply
        style: StatusStyle,
    },
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = ScryConfig::load()?;
    let store = SqliteStore::new(&config.database_url).await?;

    let project = resolve_project(&store, cli.project.as_deref()).await?;

    let Some(command) = cli.command else {
        let mut app = tui::App::new(config, store, project.id);
        return app.run().await;
    };

    match command {
        Command::Add {
            title,
            description,
            priority,
            tags,
            status,
        } => {
            let statuses = store.get_all_statuses_by_project_id(project.id).await?;
            let target_status = match status {
                Some(name) => {
                    let Some(status) = store
                        .get_status_by_project_id_and_status_name(project.id, name.clone())
                        .await?
                    else {
                        eprintln!("Status \"{}\" not found in \"{}\"", name, project.name);
                        return Ok(());
                    };
                    status
                }
                None => project
                    .entry_status_id
                    .and_then(|id| statuses.iter().find(|s| s.id == id).cloned())
                    .or_else(|| statuses.first().cloned())
                    .ok_or_else(|| AppError::Internal("project has no statuses".to_string()))?,
            };
            let position = store
                .get_all_tasks_by_status_id(target_status.id)
                .await?
                .iter()
                .map(|t| t.position)
                .max()
                .map_or(0, |p| p + 1);
            let task = store
                .create_task(TaskToCreate {
                    project_id: project.id,
                    title,
                    description: description.filter(|d| !d.is_empty()),
                    priority: priority.unwrap_or_default(),
                    status_id: target_status.id,
                    position,
                    tags: tags.map(|t| Tags::from(t.as_str())).unwrap_or_default(),
                })
                .await?;
            println!(
                "Created task {} in \"{}\" [{}]: {}",
                task.id, project.name, target_status.name, task.title
            );
        }
        Command::Move { id, status } => {
            let Some(status) = store
                .get_status_by_project_id_and_status_name(project.id, status.clone())
                .await?
            else {
                eprintln!("Status \"{}\" not found in \"{}\"", status, project.name);
                return Ok(());
            };
            let Some(task) = store.get_task_by_id(id).await? else {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            };
            let task = store
                .update_and_autoposition_task(Task {
                    status_id: status.id,
                    ..task
                })
                .await?;
            println!("Moved task {} --> \"{}\"", task.id, status.name);
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
                eprintln!("No flags provided. Use 'scry update --help' for available options.");
                return Ok(());
            }
            let Some(task) = store.get_task_by_id(id).await? else {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            };
            let status_id = match status {
                Some(status_name) => {
                    let Some(status) = store
                        .get_status_by_project_id_and_status_name(project.id, status_name.clone())
                        .await?
                    else {
                        eprintln!(
                            "Status \"{}\" not found in \"{}\"",
                            status_name, project.name
                        );
                        return Ok(());
                    };
                    status.id
                }
                None => task.status_id,
            };
            let updated = store
                .update_and_autoposition_task(Task {
                    id: task.id,
                    project_id: task.project_id,
                    title: title.unwrap_or(task.title),
                    description: match description {
                        Some(d) => Some(d).filter(|d| !d.is_empty()),
                        None => task.description,
                    },
                    priority: priority.unwrap_or(task.priority),
                    status_id,
                    position: task.position,
                    tags: tags.map(|t| Tags::from(t.as_str())).unwrap_or(task.tags),
                    created_at: task.created_at,
                })
                .await?;
            println!("Updated task {}.", updated.id);
        }
        Command::Duplicate { id } => {
            let state = ProjectState::load_from_store(&store, project.id).await?;

            let Some(task) = state.get_task_by_id(id) else {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            };

            let new_task = store.create_task(TaskToCreate::from(task)).await?;
            let status_name = state
                .get_status_by_id(new_task.status_id)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            println!(
                "Duplicated task {} as task {} in \"{}\" [{}]",
                id, new_task.id, project.name, status_name
            );
        }
        Command::Delete { id } => {
            if store.get_task_by_id(id).await?.is_none() {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            }
            store.delete_task(id).await?;
            println!("Deleted task {} from \"{}\"", id, project.name);
        }
        Command::Show { id } => {
            let state = ProjectState::load_from_store(&store, project.id).await?;

            let Some(task) = state.get_task_by_id(id) else {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            };

            let status_name = state
                .get_status_by_id(task.status_id)
                .map(|s| s.name.as_str())
                .unwrap_or("unknown");
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

            println!(
                "Priority:    p{} - {}",
                i64::from(task.priority),
                task.priority
            );
            println!("Status:      {}", status_name);
            println!(
                "Tags:        {}",
                task.tags.iter().cloned().collect::<Vec<_>>().join(" ")
            );
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
                println!("{}{} ({}):", marker, status_def.name, status_tasks.len());

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
                        line.push_str(&format!("  p{}", i64::from(task.priority)));
                    }
                    line.push_str(&format!("  {}", task.title));

                    let tags = task.tags.iter().cloned().collect::<Vec<_>>().join(" ");
                    if !tags.is_empty() {
                        line.push_str(&format!("  {}", tags));
                    }

                    println!("{}", line);
                }

                if !status_tasks.is_empty() {
                    println!();
                }
            }
        }
        Command::Note(note_cmd) => match note_cmd {
            NoteCommand::Add { task_id, contents } => {
                let state = ProjectState::load_from_store(&store, project.id).await?;

                if state.get_task_by_id(task_id).is_none() {
                    eprintln!("Task {} not found in \"{}\"", task_id, project.name);
                    return Ok(());
                }

                let note = store.create_note(task_id, contents).await?;
                println!("Added note {} to task {}.", note.id, note.task_id);
            }
        },
        Command::Project(project_cmd) => match project_cmd {
            ProjectCommand::List => {
                let projects = store.get_all_projects().await?;
                if projects.is_empty() {
                    eprintln!("No projects. Run 'scry project create <name>' to create one.");
                } else {
                    for p in &projects {
                        let marker = if p.id == project.id { "* " } else { "  " };
                        println!("  {}{}", marker, p.name);
                    }
                }
            }
            ProjectCommand::Current => {
                println!("{}", project.name);
            }
            ProjectCommand::Use { name } => {
                store.set_active_project(&name).await?;
                println!("Using project \"{}\"", name);
            }
            ProjectCommand::Create {
                name,
                template_name,
            } => {
                let template: Option<&ProjectTemplate> = match template_name {
                    Some(requested) => {
                        let Some(template) = PROJECT_TEMPLATES.iter().find(|t| t.name == requested)
                        else {
                            eprintln!("Unknown template name: \"{}\".", requested);
                            return Ok(());
                        };
                        Some(template)
                    }
                    None => None,
                };

                let project = create_project(&store, name, template.copied()).await?;

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
            ProjectCommand::Delete { name, force } => {
                if !force {
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
                store.delete_project(name.clone()).await?;
                println!("Deleted project \"{}\"", name);
                let new_active = store.get_active_project().await?;
                if new_active.name != name {
                    println!("Using project \"{}\"", new_active.name);
                }
            }
            ProjectCommand::Rename { old_name, new_name } => {
                store
                    .update_project(Project {
                        name: new_name.clone(),
                        ..project
                    })
                    .await?;
                println!("Renamed project \"{}\" --> \"{}\"", old_name, new_name);
            }
            ProjectCommand::Status(status_cmd) => match status_cmd {
                StatusCommand::List => {
                    let statuses = store.get_all_statuses_by_project_id(project.id).await?;
                    println!("Statuses for \"{}\":", project.name);
                    for s in &statuses {
                        println!("  {}", s.name);
                    }
                }
                StatusCommand::Add { name } => {
                    let statuses = store.get_all_statuses_by_project_id(project.id).await?;
                    let status = store
                        .create_status(
                            project.id,
                            name,
                            statuses.len() as i32,
                            None,
                            StatusStyle::None,
                        )
                        .await?;
                    println!(
                        "Added status \"{}\" to project \"{}\"",
                        status.name, project.name
                    );
                }
                StatusCommand::Remove { name } => {
                    if let Some(status) = store
                        .get_status_by_project_id_and_status_name(project.id, name.clone())
                        .await?
                    {
                        let tasks_in_status = store.get_all_tasks_by_status_id(status.id).await?;
                        if tasks_in_status.is_empty() {
                            store.delete_status(status.id).await?;
                            println!(
                                "Removed status \"{}\" from project \"{}\"",
                                name, project.name
                            );
                        } else {
                            eprintln!(
                                "Cannot delete status with active tasks. Status \"{}\" contains {} tasks",
                                status.name,
                                tasks_in_status.len()
                            );
                        }
                    }
                }
                StatusCommand::Rename { old_name, new_name } => {
                    if let Some(status) = store
                        .get_status_by_project_id_and_status_name(project.id, old_name.clone())
                        .await?
                    {
                        store
                            .update_status(Status {
                                name: new_name.clone(),
                                ..status
                            })
                            .await?;
                        println!(
                            "Renamed status \"{}\" --> \"{}\" in project \"{}\"",
                            old_name, new_name, project.name
                        );
                    }
                }
                StatusCommand::SetStyle { name, style } => {
                    if let Some(status) = store
                        .get_status_by_project_id_and_status_name(project.id, name.clone())
                        .await?
                    {
                        store.update_status(Status { style, ..status }).await?;
                        println!(
                            "Set style of status \"{}\" to \"{}\" in project \"{}\"",
                            name, style, project.name
                        );
                    } else {
                        eprintln!("Status \"{}\" not found in \"{}\"", name, project.name);
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
            .ok_or_else(|| AppError::Internal(format!("project '{}' not found", name)))?;
        Ok(project)
    } else {
        let project = store.get_active_project().await?;
        Ok(project)
    }
}

async fn create_project(
    store: &dyn TaskStore,
    name: String,
    template: Option<ProjectTemplate>,
) -> Result<Project, AppError> {
    let project = store
        .create_project(name, None, TaskSortingMode::default(), false)
        .await?;

    let Some(template) = template else {
        return Ok(project);
    };

    let mut statuses = vec![];
    for status in template.statuses {
        statuses.push(
            store
                .create_status(
                    project.id,
                    status.name.to_string(),
                    status.position,
                    status.color,
                    status.style,
                )
                .await?,
        );
    }

    let entry_status_id = template
        .entry_status_name
        .and_then(|name| statuses.iter().find(|s| s.name == name).map(|s| s.id));

    Ok(store
        .update_project(Project {
            entry_status_id,
            task_sorting_mode: template.task_sorting_mode,
            show_priority: template.show_priority,
            ..project
        })
        .await?)
}
