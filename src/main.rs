mod config;
mod error;
mod models;
mod store;
mod tui;

use chrono::Local;
use clap::{Parser, Subcommand};
use config::ScryConfig;
use error::AppError;
use store::{TaskStore, sqlite::SqliteStore};

use crate::{
    models::{Priority, Project, Status, StatusStyle, Tags, Task, TaskSortingMode},
    store::TaskToCreate,
};

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
        /// Move the task to a new status
        #[arg(long)]
        status: Option<String>,
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
    },
    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommand),
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
        Command::Add { title } => {
            let statuses = store.get_all_statuses_by_project_id(project.id).await?;
            let first_status = statuses
                .first()
                .ok_or_else(|| AppError::Internal("project has no statuses".to_string()))?;
            let position = store
                .get_all_tasks_by_status_id(first_status.id)
                .await?
                .len() as i32;
            let task = store
                .create_task(TaskToCreate {
                    project_id: project.id,
                    title,
                    description: None,
                    priority: Priority::default(),
                    status_id: first_status.id,
                    position,
                    tags: Tags::default(),
                })
                .await?;
            println!(
                "Created task {} in \"{}\" [{}]: {}",
                task.id,
                project.name,
                statuses
                    .iter()
                    .find(|s| s.id == task.status_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("?"),
                task.title
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
        Command::Update { id, status } => {
            let Some(status_name) = status else {
                eprintln!("No flags provided. Use 'scry update --help' for available options.");
                return Ok(());
            };
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
            println!("Updated task {}: status --> \"{}\"", task.id, &status_name);
        }
        Command::Delete { id } => {
            if store.get_task_by_id(id).await?.is_none() {
                eprintln!("Task {} not found in \"{}\"", id, project.name);
                return Ok(());
            }
            store.delete_task(id).await?;
            println!("Deleted task {} from \"{}\"", id, project.name);
        }
        Command::Show { id } => match store.get_task_by_id(id).await? {
            Some(task) => {
                let status_defs = store.get_all_statuses_by_project_id(project.id).await?;
                let status_name = status_defs
                    .iter()
                    .find(|s| s.id == task.status_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("unknown");

                println!("Task {}", task.id);
                println!("  Project:   {}", project.name);
                println!("  Title:     {}", task.title);
                println!("  Status:    {}", status_name);
                println!(
                    "  Created:   {}",
                    task.created_at
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %I:%M %p %Z")
                );
            }
            None => eprintln!("Task {} not found in \"{}\"", id, project.name),
        },
        Command::List { status } => {
            let tasks = match &status {
                Some(name) => match store
                    .get_status_by_project_id_and_status_name(project.id, name.clone())
                    .await?
                {
                    Some(status_def) => store.get_all_tasks_by_status_id(status_def.id).await?,
                    None => vec![],
                },
                None => store.get_all_tasks_by_project_id(project.id).await?,
            };
            let statuses = store.get_all_statuses_by_project_id(project.id).await?;

            println!("project \"{}\"\n", project.name);

            if tasks.is_empty() {
                println!("No tasks.");
                return Ok(());
            }

            for status_def in &statuses {
                let status_tasks: Vec<_> = tasks
                    .iter()
                    .filter(|t| t.status_id == status_def.id)
                    .collect();

                if let Some(ref filter) = status
                    && status_def.name != *filter
                {
                    continue;
                }

                println!("{} ({}):", status_def.name, status_tasks.len());
                for task in &status_tasks {
                    let icon = if status_def.style == StatusStyle::Checked
                        || status_def.style == StatusStyle::Strikethrough
                    {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    println!("  {}  {}  {}", task.id, icon, task.title);
                }
                if !status_tasks.is_empty() {
                    println!();
                }
            }
        }
        Command::Project(project_cmd) => match project_cmd {
            ProjectCommand::List => {
                let projects = store.get_all_projects().await?;
                if projects.is_empty() {
                    println!("No projects. Run 'scry project create <name>' to create one.");
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
            ProjectCommand::Create { name } => {
                let project = store
                    .create_project(name, None, TaskSortingMode::default(), false)
                    .await?;
                println!("Created project \"{}\"", project.name);
                println!(
                    "Note: this project has no statuses yet. Add one with 'scry project status add <name>'."
                );
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
