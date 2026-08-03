mod error;
mod models;
mod store;

use clap::{Parser, Subcommand};
use error::AppError;
use store::{sqlite::SqliteStore, TaskStore};

#[derive(Parser)]
#[command(name = "scry", about = "A task manager for the terminal")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task
    Add {
        /// The task description
        description: String,
    },
    /// Mark a task as complete
    Complete {
        /// The task ID to complete
        id: i64,
    },
    /// Delete a task
    Delete {
        /// The task ID to delete
        id: i64,
    },
    /// List all tasks
    List,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let store = SqliteStore::new().await?;

    match cli.command {
        Command::Add { description } => {
            let task = store.add(&description).await?;
            println!("✓ Added task #{}: {}", task.id, task.description);
        }
        Command::Complete { id } => {
            match store.complete(id).await? {
                Some(task) => println!("✓ Completed task #{}: {}", task.id, task.description),
                None => eprintln!("Task #{} not found or already completed", id),
            }
        }
        Command::Delete { id } => {
            if store.delete(id).await? {
                println!("✓ Deleted task #{}", id);
            } else {
                eprintln!("Task #{} not found", id);
            }
        }
        Command::List => {
            let tasks = store.list_all().await?;
            if tasks.is_empty() {
                println!("No tasks yet. Add one with: scry add \"your task\"");
                return Ok(());
            }

            // calculate padding for right-aligned IDs
            let max_id_width = tasks.iter().map(|t| t.id).max().unwrap_or(1).to_string().len();

            for task in &tasks {
                let checkbox = if task.is_complete { "[x]" } else { "[ ]" };
                println!(
                    "{:>width$}  {}  {}",
                    task.id,
                    checkbox,
                    task.description,
                    width = max_id_width
                );
            }
        }
    }

    Ok(())
}
