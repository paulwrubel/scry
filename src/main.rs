mod client_state;
mod color;
mod config;
mod docs;
mod error;
mod http_auth;
mod postgres;
mod server;
mod service;
mod skill;
mod state;

use crate::color::ColorChoice;
use crate::postgres::PostgresStore;
use crate::service::{EntryStatusUpdate, ProjectService, ProjectUpdate, TaskInput};
use crate::skill::Harness;
use crate::state::{DATETIME_FORMAT_STR, ProjectState};
use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand};
use config::{ScryConfig, ScrySecrets};
use error::{AppError, ServiceError};
use scry_core::backup::{self, BACKUP_VERSION, Backup, ImportMode};
use scry_core::models::{
    Color, PROJECT_TEMPLATES, Priority, Project, ProjectTemplate, StatusStyle, Tags, Task,
    TaskSortingMode,
};
use scry_core::store::ArcStore;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

    /// Read configuration from this file instead of the discovered default
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Read secrets from this file instead of the discovered default
    #[arg(long, value_name = "PATH")]
    secrets: Option<PathBuf>,

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
    /// Export all data (or a single project with the global -p/--project) as JSON to stdout
    Export,
    /// Import a JSON backup from stdin
    Import {
        /// Replace the existing project when the backup has a project with the same name
        #[arg(long, conflicts_with = "skip_existing_projects")]
        replace_existing_projects: bool,
        /// Keep the existing project and skip the backup's project of the same name
        #[arg(long, conflicts_with = "replace_existing_projects")]
        skip_existing_projects: bool,
    },
    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Print the bundled documentation (the README) to stdout
    Docs,
    /// Manage the agent skill that teaches coding agents to drive scry
    #[command(subcommand)]
    Skill(SkillCommand),
    /// Serve the REST API over HTTP
    Serve {
        /// The address to bind, for example `127.0.0.1:9000`
        #[arg(long, value_name = "ADDR")]
        http: String,
    },
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
    /// Show a project's details
    Show {
        /// The project name (defaults to the active project)
        name: Option<String>,
    },
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
        /// The project description
        #[arg(long)]
        description: Option<String>,
        /// The project name
        name: String,
    },
    /// Update project properties
    Update {
        /// New project name
        #[arg(long)]
        name: Option<String>,
        /// New description (empty string clears it)
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
        /// Show the priority level in task listings
        #[arg(long, conflicts_with = "hide_priority")]
        show_priority: bool,
        /// Hide the priority level in task listings
        #[arg(long, conflicts_with = "show_priority")]
        hide_priority: bool,
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

#[derive(Subcommand)]
enum SkillCommand {
    /// Install the scry agent skill
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["harness", "dir"])
    ))]
    Install {
        /// Install at a harness's standard user-global location
        #[arg(long, value_enum)]
        harness: Option<Harness>,
        /// Install into a custom skills directory (writes <DIR>/scry/SKILL.md)
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
        /// Overwrite an existing skill file
        #[arg(long)]
        force: bool,
    },
    /// Remove an installed scry agent skill
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["harness", "dir"])
    ))]
    Uninstall {
        /// Remove from a harness's standard user-global location
        #[arg(long, value_enum)]
        harness: Option<Harness>,
        /// Remove from a custom skills directory (<DIR>/scry/SKILL.md)
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    /// Print the agent skill to stdout without writing anything
    Print,
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
    if output_json
        && (cli.command.is_none()
            || matches!(
                cli.command.as_ref(),
                Some(Command::Serve { .. } | Command::Docs | Command::Skill(_))
            ))
    {
        return Err(AppError::Usage(
            "--json is exclusive to cli commands".to_string(),
        ));
    }

    if output_json {
        ColorChoice::Never.apply();
    } else {
        cli.color.apply();
    }

    // `scry docs` only prints the embedded README: no database or active project involved.
    if matches!(cli.command.as_ref(), Some(Command::Docs)) {
        print!("{}", docs::README_MD);
        return Ok(());
    }

    // Skill commands are filesystem-only: they never touch the database or active project.
    if let Some(Command::Skill(command)) = &cli.command {
        return run_skill(command);
    }

    if cli.command.is_none() {
        Cli::command()
            .print_help()
            .map_err(|error| AppError::Internal(format!("failed to print help: {error}")))?;
        println!();
        println!(
            "The interactive TUI has been removed. Run `scry serve` for the REST API, or `scry --help` for available commands."
        );
        return Ok(());
    }

    let config = ScryConfig::load(cli.config.as_deref())?;
    let secrets = ScrySecrets::load(cli.secrets.as_deref())?;
    let database_url = secrets.resolved_database_url(&config)?;
    let store = connect_store(&database_url).await?;

    if let Some(Command::Serve { http }) = &cli.command {
        return server::serve_http(store, http, secrets.api_token()).await;
    }

    let service = ProjectService::new(store.as_ref());

    // These commands create, list, select, or remove projects, or move data in
    // and out; none of them needs an active project, so they run even when the
    // active project cannot be resolved.
    if let Some(Command::Project(subcommand)) = cli.command.as_ref() {
        match subcommand {
            ProjectCommand::List => {
                let projects = store.get_all_projects().await?;
                if output_json {
                    print_json(&projects)?;
                } else if projects.is_empty() {
                    eprintln!("No projects. Run 'scry project create <name>' to create one.");
                } else {
                    let active_id = client_state::active_project(store.as_ref(), &config)
                        .await
                        .ok()
                        .map(|project| project.id);
                    for p in &projects {
                        let marker = if Some(p.id) == active_id { "* " } else { "  " };
                        println!("  {}{}", marker, p.name);
                    }
                }
                return Ok(());
            }
            ProjectCommand::Show { name } => {
                let project = resolve_project(
                    store.clone(),
                    &config,
                    name.as_deref().or(cli.project.as_deref()),
                )
                .await?;
                if output_json {
                    print_json(&project)?;
                } else {
                    let statuses = store.get_all_statuses_by_project_id(project.id).await?;
                    let entry_status_name = project
                        .entry_status_id
                        .and_then(|id| statuses.iter().find(|status| status.id == id))
                        .map(|status| status.name.as_str())
                        .unwrap_or("(first status)");
                    println!("{}", project.name);
                    if let Some(description) = &project.description {
                        for line in description.lines() {
                            println!("    {}", line);
                        }
                    }
                    println!();
                    println!("Entry status: {}", entry_status_name);
                    println!("Sort mode:    {}", project.task_sorting_mode);
                    println!(
                        "Priority:     {}",
                        if project.show_priority {
                            "shown"
                        } else {
                            "hidden"
                        }
                    );
                    println!(
                        "Created at:   {}",
                        project
                            .created_at
                            .with_timezone(&Local)
                            .format(DATETIME_FORMAT_STR)
                    );
                }
                return Ok(());
            }
            ProjectCommand::Create {
                name,
                template_name,
                description,
            } => {
                let template: Option<&ProjectTemplate> = match template_name {
                    Some(requested) => {
                        let Some(template) =
                            PROJECT_TEMPLATES.iter().find(|t| t.name == *requested)
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

                let project = service
                    .create_project(name.clone(), description.clone(), template.copied())
                    .await?;

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
                return Ok(());
            }
            ProjectCommand::Use { name } => {
                let active =
                    client_state::set_active_project(store.as_ref(), &config, name).await?;
                if output_json {
                    print_json(&active)?;
                } else {
                    println!("Using project \"{}\"", name);
                }
                return Ok(());
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
                let removed = service.delete_project(name).await?;
                if output_json {
                    print_json(&removed)?;
                } else {
                    println!("Deleted project \"{}\"", name);
                    if let Ok(new_active) =
                        client_state::active_project(store.as_ref(), &config).await
                        && new_active.name != *name
                    {
                        println!("Using project \"{}\"", new_active.name);
                    }
                }
                return Ok(());
            }
            _ => {}
        }
    }

    if let Some(Command::Export) = cli.command.as_ref() {
        let export_project_id = match cli.project.as_deref() {
            Some(name) => Some(
                store
                    .get_project_by_name(name)
                    .await?
                    .ok_or_else(|| AppError::Usage(format!("project '{name}' not found")))?
                    .id,
            ),
            None => None,
        };

        let mut backup = backup::export(store.as_ref(), export_project_id).await?;
        backup.app_version = env!("CARGO_PKG_VERSION").to_string();
        let json = serde_json::to_string_pretty(&backup)
            .map_err(|e| AppError::Internal(format!("failed to serialize JSON: {e}")))?;
        println!("{json}");
        return Ok(());
    }

    if let Some(Command::Import {
        replace_existing_projects,
        skip_existing_projects,
    }) = cli.command.as_ref()
    {
        use std::io::Read;

        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|e| AppError::Internal(format!("failed to read stdin: {e}")))?;

        let backup: Backup = serde_json::from_str(&input)
            .map_err(|e| AppError::Usage(format!("invalid backup JSON: {e}")))?;

        if backup.version != BACKUP_VERSION {
            return Err(AppError::Usage(format!(
                "unsupported backup version {}; this build supports version {BACKUP_VERSION}",
                backup.version
            )));
        }

        let mode = if *replace_existing_projects {
            ImportMode::Replace
        } else if *skip_existing_projects {
            ImportMode::Skip
        } else {
            ImportMode::Fail
        };

        let plan = backup::plan_import(store.as_ref(), &backup, mode).await?;
        let report = store.import(plan).await?;

        if output_json {
            let document = serde_json::json!({
                "projects": report.projects,
                "statuses": report.statuses,
                "tasks": report.tasks,
                "notes": report.notes,
            });
            print_json(&document)?;
        } else {
            println!(
                "Imported {} project(s), {} status(es), {} task(s), {} note(s)",
                report.projects, report.statuses, report.tasks, report.notes
            );
        }
        return Ok(());
    }

    let project = resolve_project(store.clone(), &config, cli.project.as_deref()).await?;

    let command = cli.command.expect("no subcommand is handled above");

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
            let state = ProjectState::load_from_store(store.as_ref(), project.id).await?;

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
            let mut state = ProjectState::load_from_store(store.as_ref(), project.id).await?;
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
                        StatusStyle::None | StatusStyle::Strikethrough | StatusStyle::Hidden => "",
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
            ProjectCommand::List
            | ProjectCommand::Create { .. }
            | ProjectCommand::Show { .. }
            | ProjectCommand::Use { .. }
            | ProjectCommand::Delete { .. } => {
                unreachable!("handled before the active project is resolved")
            }
            ProjectCommand::Current => {
                if output_json {
                    print_json(&project)?;
                } else {
                    println!("{}", project.name);
                }
            }
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
                    return Err(AppError::Usage(
                        "No flags provided. Use 'scry project update --help' for available options."
                            .to_string(),
                    ));
                }
                let entry_status = if clear_entry_status {
                    EntryStatusUpdate::Clear
                } else if let Some(status_name) = entry_status {
                    let status = service.get_status_by_name(&project, &status_name).await?;
                    EntryStatusUpdate::Set(status.id)
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
                let updated = service
                    .update_project(
                        &project,
                        ProjectUpdate {
                            name,
                            description,
                            entry_status,
                            task_sorting_mode: sort,
                            show_priority,
                        },
                    )
                    .await?;
                if output_json {
                    print_json(&updated)?;
                } else {
                    println!("Updated project \"{}\".", updated.name);
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
                        print_json(moved.as_ref().unwrap_or(&status))?;
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
                        print_json(moved.as_ref().unwrap_or(&status))?;
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
        Command::Export => unreachable!("handled before the active project is resolved"),
        Command::Import { .. } => unreachable!("handled before the active project is resolved"),
        Command::Docs => unreachable!("docs is handled before database setup"),
        Command::Skill(_) => unreachable!("skill commands are handled before database setup"),
        Command::Serve { .. } => {
            unreachable!("the REST server is handled before database-backed commands")
        }
    }

    Ok(())
}

/// Handle `scry skill` commands. These operate on the filesystem only and never
/// touch the database or active project.
fn run_skill(command: &SkillCommand) -> Result<(), AppError> {
    match command {
        SkillCommand::Install {
            harness,
            dir,
            force,
        } => {
            let path = skill_target(*harness, dir.as_deref())?;
            let path = skill::install(&path, *force)?;
            println!("Installed scry skill to {}", path.display());
        }
        SkillCommand::Uninstall { harness, dir } => {
            let path = skill_target(*harness, dir.as_deref())?;
            let path = skill::uninstall(&path)?;
            println!("Uninstalled scry skill from {}", path.display());
        }
        SkillCommand::Print => {
            print!("{}", skill::SKILL_MD);
        }
    }

    Ok(())
}

/// Resolve the skill file path from the mutually exclusive `--harness`/`--dir` flags.
fn skill_target(harness: Option<Harness>, dir: Option<&Path>) -> Result<PathBuf, AppError> {
    match (harness, dir) {
        (Some(harness), None) => Ok(harness.global_path()),
        (None, Some(dir)) => Ok(skill::path_in_skills_dir(dir)),
        _ => Err(AppError::Usage(
            "specify exactly one of --harness or --dir".to_string(),
        )),
    }
}

/// Open the Postgres store for the given database URL.
///
/// Only Postgres URLs (`postgres://` and `postgresql://`) are supported.
async fn connect_store(database_url: &str) -> Result<ArcStore, AppError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Ok(Arc::new(PostgresStore::new(database_url).await?))
    } else {
        Err(AppError::Config(format!(
            "unsupported database URL {database_url}: only postgres:// and postgresql:// are supported"
        )))
    }
}

/// Resolve which project to use: --project flag takes precedence over active project.
async fn resolve_project(
    store: ArcStore,
    config: &ScryConfig,
    flag: Option<&str>,
) -> Result<Project, AppError> {
    if let Some(name) = flag {
        let project = store
            .get_project_by_name(name)
            .await?
            .ok_or_else(|| AppError::Usage(format!("project '{}' not found", name)))?;
        Ok(project)
    } else {
        let project = client_state::active_project(store.as_ref(), config).await?;
        Ok(project)
    }
}
