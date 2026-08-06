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
        /// The task description
        description: String,
    },
    /// Move a task to a new state (alias for update --state)
    Move {
        /// The task ID
        id: i64,
        /// The target state
        state: String,
    },
    /// Update task properties
    Update {
        /// The task ID
        id: i64,
        /// Move the task to a new state
        #[arg(long)]
        state: Option<String>,
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
        /// Show only tasks in a specific state
        #[arg(long)]
        state: Option<String>,
    },
    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommand),
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// List all projects
    List,
    /// Create a new project
    Create {
        /// The project name
        name: String,
    },
    /// Delete a project and all its tasks
    Delete {
        /// The project name
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Set the active project
    Use {
        /// The project name
        name: String,
    },
    /// Show the currently active project
    Current,
    /// Manage states within a project
    #[command(subcommand)]
    State(StateCommand),
}

#[derive(Subcommand)]
enum StateCommand {
    /// List states for a project
    List,
    /// Add a new state
    Add {
        /// The state name
        name: String,
    },
    /// Remove a state from a project
    Remove {
        /// The state name
        name: String,
        /// Move tasks to the first remaining state
        #[arg(short, long)]
        force: bool,
    },
    /// Rename a state
    Rename {
        /// The current state name
        old_name: String,
        /// The new state name
        new_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = ScryConfig::load()?;
    let db_url = config.resolve_database_url();
    let store = SqliteStore::new(&db_url).await?;

    let Some(command) = cli.command else {
        let mut app = tui::App::from_store(&store).await?;
        return app.run(&store).await;
    };

    let (project_id, project_name) = resolve_project(&store, cli.project.as_deref()).await?;

    match command {
        Command::Add { description } => {
            let task = store.add_task(&description, project_id).await?;
            println!(
                "Created task {} in \"{}\" [{}]: {}",
                task.id, project_name, task.state_name, task.title
            );
        }
        Command::Move { id, state } => match store.move_task(id, project_id, &state).await? {
            Some(_) => println!("Moved task {} --> \"{}\"", id, state),
            None => eprintln!("Task {} not found in \"{}\"", id, project_name),
        },
        Command::Update { id, state } => {
            let Some(state_name) = state else {
                eprintln!("No flags provided. Use 'scry update --help' for available options.");
                return Ok(());
            };
            match store.update_task(id, project_id, Some(&state_name)).await? {
                Some(task) => {
                    println!("Updated task {}: state --> \"{}\"", task.id, &state_name);
                }
                None => eprintln!("Task {} not found in \"{}\"", id, project_name),
            }
        }
        Command::Delete { id } => {
            if store.delete_task(id, project_id).await? {
                println!("Deleted task {} from \"{}\"", id, project_name);
            } else {
                eprintln!("Task {} not found in \"{}\"", id, project_name);
            }
        }
        Command::Show { id } => match store.show_task(id, project_id).await? {
            Some(task) => {
                println!("Task {}", task.id);
                println!("  Project:   {}", project_name);
                println!("  Title:     {}", task.title);
                println!("  State:     {}", task.state_name);
                println!(
                    "  Created:   {}",
                    task.created_at
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %I:%M %p %Z")
                );
                match task.completed_at {
                    Some(ts) => println!(
                        "  Completed: {}",
                        ts.with_timezone(&Local).format("%Y-%m-%d %I:%M %p %Z")
                    ),
                    None => println!("  Completed: -"),
                }
            }
            None => eprintln!("Task {} not found in \"{}\"", id, project_name),
        },
        Command::List { state } => {
            let tasks = store.list_tasks(project_id, state.as_deref()).await?;
            let states = store.list_states(project_id).await?;

            println!("project \"{}\"\n", project_name);

            if tasks.is_empty() {
                println!("No tasks.");
                return Ok(());
            }

            for state_def in &states {
                let state_tasks: Vec<_> = tasks
                    .iter()
                    .filter(|t| t.state_name == state_def.name)
                    .collect();

                if let Some(ref filter) = state
                    && state_def.name != *filter
                {
                    continue;
                }

                println!("{} ({}):", state_def.name, state_tasks.len());
                for task in &state_tasks {
                    let icon = if task.completed_at.is_some() {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    println!("  {}  {}  {}", task.id, icon, task.title);
                }
                if !state_tasks.is_empty() {
                    println!();
                }
            }
        }
        Command::Project(project_cmd) => match project_cmd {
            ProjectCommand::List => {
                let projects = store.list_projects().await?;
                if projects.is_empty() {
                    println!("No projects. Run 'scry project create <name>' to create one.");
                } else {
                    for p in &projects {
                        let marker = if p.id == project_id { "* " } else { "  " };
                        println!("  {}{}", marker, p.name);
                    }
                }
            }
            ProjectCommand::Create { name } => {
                let project = store.create_project(&name).await?;
                println!(
                    "Created project \"{}\" with states: todo, done",
                    project.name
                );
                println!("Using project \"{}\"", project.name);
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
                store.delete_project(&name).await?;
                println!("Deleted project \"{}\"", name);
                let new_active = store.get_active_project().await?;
                if new_active.name != name {
                    println!("Using project \"{}\"", new_active.name);
                }
            }
            ProjectCommand::Use { name } => {
                store.set_active_project(&name).await?;
                println!("Using project \"{}\"", name);
            }
            ProjectCommand::Current => {
                println!("{}", project_name);
            }
            ProjectCommand::State(state_cmd) => match state_cmd {
                StateCommand::List => {
                    let states = store.list_states(project_id).await?;
                    println!("States for \"{}\":", project_name);
                    for s in &states {
                        println!("  {}", s.name);
                    }
                }
                StateCommand::Add { name } => {
                    let state = store.add_state(project_id, &name).await?;
                    println!(
                        "Added state \"{}\" to project \"{}\"",
                        state.name, project_name
                    );
                }
                StateCommand::Remove { name, force } => {
                    store.remove_state(project_id, &name, force).await?;
                    println!(
                        "Removed state \"{}\" from project \"{}\"",
                        name, project_name
                    );
                }
                StateCommand::Rename { old_name, new_name } => {
                    store.rename_state(project_id, &old_name, &new_name).await?;
                    println!(
                        "Renamed state \"{}\" --> \"{}\" in project \"{}\"",
                        old_name, new_name, project_name
                    );
                }
            },
        },
    }

    Ok(())
}

/// Resolve which project to use: --project flag takes precedence over active project.
async fn resolve_project(
    store: &SqliteStore,
    flag: Option<&str>,
) -> Result<(i64, String), AppError> {
    if let Some(name) = flag {
        let project = store
            .get_project_by_name(name)
            .await?
            .ok_or_else(|| AppError::Internal(format!("project '{}' not found", name)))?;
        Ok((project.id, project.name))
    } else {
        let project = store.get_active_project().await?;
        Ok((project.id, project.name))
    }
}
