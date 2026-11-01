mod client;
mod client_state;
mod color;
mod config;
mod docs;
mod error;
mod http_auth;
mod output;
mod server;
mod service;
mod skill;
mod state;
mod store;

use crate::color::ColorChoice;
use crate::service::{ColorChange, ProjectService, ProjectSetup, UpdateProject, UpdateSection};
use crate::skill::Harness;
use crate::store::{ArcStore, PostgresStore};
use clap::{CommandFactory, Parser, Subcommand};
use config::{ScryConfig, ScrySecrets};
use error::AppError;
use scry_core::backup::{self, ImportMode};
use scry_core::ids::Handle;
use scry_core::models::{
    CloseReason, Color, PROJECT_TEMPLATES, Priority, Project, ProjectTemplate, TaskSortingMode,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "scry", about = "A task manager for the terminal", version)]
struct Cli {
    /// Target a specific project (overrides the active project)
    #[arg(short = 'p', long = "project")]
    project: Option<String>,

    /// Talk to this server URL instead of the configured server_url
    #[arg(long, value_name = "URL")]
    server: Option<String>,

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
        /// Place the task in an existing section
        #[arg(long, conflicts_with = "new_section")]
        section: Option<String>,
        /// Create a new section for the task
        #[arg(long, conflicts_with = "section")]
        new_section: Option<String>,
    },
    /// Move a task to a section
    Move {
        /// The task ID or a unique handle prefix
        id: Handle,
        /// The target section
        section: Option<String>,
        /// Move the task to No section
        #[arg(long, conflicts_with = "section")]
        no_section: bool,
    },
    /// Update task properties
    Update {
        /// The task ID or a unique handle prefix
        id: Handle,
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
        /// Move the task to an existing section
        #[arg(long, conflicts_with_all = ["new_section", "no_section"])]
        section: Option<String>,
        /// Move the task to a new section
        #[arg(long, conflicts_with_all = ["section", "no_section"])]
        new_section: Option<String>,
        /// Move the task to No section
        #[arg(long, conflicts_with_all = ["section", "new_section"])]
        no_section: bool,
    },
    /// Duplicate an existing task
    Duplicate {
        /// The task ID or a unique handle prefix
        id: Handle,
    },
    /// Delete a task
    Delete {
        /// The task ID or a unique handle prefix
        id: Handle,
    },
    /// Close tasks with a reason
    Close {
        /// The task IDs or unique handle prefixes
        #[arg(required = true, num_args = 1..)]
        ids: Vec<Handle>,
        /// The reason for closing
        #[arg(long)]
        reason: CloseReason,
    },
    /// Open closed tasks
    Open {
        /// The task IDs or unique handle prefixes
        #[arg(required = true, num_args = 1..)]
        ids: Vec<Handle>,
    },
    /// Show full details for a task
    Show {
        /// The task ID or a unique handle prefix
        id: Handle,
    },
    /// List tasks in the active project
    List {
        /// Show only tasks in a specific section
        #[arg(long)]
        section: Option<String>,
        /// Case-insensitive substring filter over task titles and tags
        #[arg(long)]
        search: Option<String>,
        /// Include closed tasks after the open sections
        #[arg(long)]
        include_closed: bool,
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
    /// Manage sections in the active or -p project
    #[command(subcommand)]
    Section(SectionCommand),
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
        /// The task ID or a unique handle prefix
        task_id: Handle,
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
}

#[derive(Subcommand)]
enum SectionCommand {
    /// List sections for a project
    List,
    /// Add a new section
    Add {
        /// The section name
        name: String,
        /// The section color
        #[arg(long)]
        color: Option<Color>,
    },
    /// Remove a section, moving its tasks to No section
    Remove {
        /// The section name
        name: String,
    },
    /// Rename a section
    Rename {
        /// The current section name
        old_name: String,
        /// The new section name
        new_name: String,
    },
    /// Merge a section into another section
    Merge {
        /// The section to merge from
        source: String,
        /// The section to merge into
        target: String,
    },
    /// Set the color for a section
    SetColor {
        /// The section name
        name: String,
        /// The color to apply
        color: Color,
    },
    /// Reset the color for a section
    ResetColor {
        /// The section name
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
        return Ok(());
    }

    let config = ScryConfig::load(cli.config.as_deref())?;
    let secrets = ScrySecrets::load(cli.secrets.as_deref())?;

    // Commands that run entirely over HTTP, before any database connection.
    match cli.command.as_ref() {
        Some(Command::Project(ProjectCommand::List)) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_list(&client, &config, output_json).await;
        }
        Some(Command::List {
            section,
            search,
            include_closed,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::list(
                &client,
                &config,
                cli.project.as_deref(),
                section.as_deref(),
                search.as_deref(),
                *include_closed,
                output_json,
            )
            .await;
        }
        Some(Command::Show { id }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::show(&client, id.as_str(), output_json).await;
        }
        Some(Command::Add {
            title,
            description,
            priority,
            tags,
            section,
            new_section,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::add(
                &client,
                &config,
                client::AddTask {
                    project: cli.project.as_deref(),
                    title,
                    description: description.as_deref(),
                    priority: *priority,
                    tags: tags.as_deref(),
                    section: section.as_deref(),
                    new_section: new_section.as_deref(),
                },
                output_json,
            )
            .await;
        }
        Some(Command::Update {
            id,
            title,
            description,
            priority,
            tags,
            section,
            new_section,
            no_section,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::update(
                &client,
                client::UpdateTask {
                    handle: id.as_str(),
                    title: title.as_deref(),
                    description: description.as_deref(),
                    priority: *priority,
                    tags: tags.as_deref(),
                    section: section.as_deref(),
                    new_section: new_section.as_deref(),
                    no_section: *no_section,
                },
                output_json,
            )
            .await;
        }
        Some(Command::Move {
            id,
            section,
            no_section,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::move_task(
                &client,
                client::MoveTask {
                    handle: id.as_str(),
                    section: section.as_deref(),
                    no_section: *no_section,
                },
                output_json,
            )
            .await;
        }
        Some(Command::Duplicate { id }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::duplicate(&client, id.as_str(), output_json).await;
        }
        Some(Command::Delete { id }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::delete(&client, id.as_str(), output_json).await;
        }
        Some(Command::Close { ids, reason }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::close_tasks(&client, ids, *reason, output_json).await;
        }
        Some(Command::Open { ids }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::open_tasks(&client, ids, output_json).await;
        }
        Some(Command::Note(NoteCommand::Add { task_id, contents })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::note_add(&client, task_id.as_str(), contents, output_json).await;
        }
        Some(Command::Project(ProjectCommand::Show { name })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_show(
                &client,
                &config,
                name.as_deref(),
                cli.project.as_deref(),
                output_json,
            )
            .await;
        }
        Some(Command::Project(ProjectCommand::Current)) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_current(&client, &config, output_json).await;
        }
        Some(Command::Project(ProjectCommand::Use { name })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_use(&client, &config, name, output_json).await;
        }
        _ => {}
    }

    let database_url = secrets.resolved_database_url(&config)?;
    let store = connect_store(&database_url).await?;
    let service = ProjectService::new(store);

    if let Some(Command::Serve { http }) = &cli.command {
        return server::serve_http(service, http, secrets.api_token()).await;
    }

    // These commands create, list, select, or remove projects, or move data in
    // and out; none of them needs an active project, so they run even when the
    // active project cannot be resolved.
    if let Some(Command::Project(subcommand)) = cli.command.as_ref() {
        match subcommand {
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

                let setup = match template {
                    Some(template) => ProjectSetup::Template(*template),
                    None => ProjectSetup::Defaults,
                };
                let project = service
                    .create_project(name.clone(), description.clone(), setup)
                    .await?;

                if output_json {
                    output::print_json(&project)?;
                } else {
                    println!("Created project \"{}\"", project.name);
                    if template.is_none() {
                        println!(
                            "Note: sections are optional. Add one with 'scry section add <name>'."
                        );
                    }
                    println!(
                        "Make it active with 'scry project use \"{}\"'.",
                        project.name
                    );
                }
                return Ok(());
            }
            ProjectCommand::Delete { name, force } => {
                let project = service
                    .get_project_by_name(name)
                    .await?
                    .ok_or_else(|| AppError::Usage(format!("project '{name}' not found")))?;
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
                let removed = service.delete_project_by_id(project.id).await?;
                if output_json {
                    output::print_json(&removed)?;
                } else {
                    println!("Deleted project \"{}\"", name);
                    if let Ok(new_active) = client_state::active_project(&service, &config).await
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
                service
                    .get_project_by_name(name)
                    .await?
                    .ok_or_else(|| AppError::Usage(format!("project '{name}' not found")))?
                    .id,
            ),
            None => None,
        };

        let mut backup = service.export(export_project_id).await?;
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

        let backup = backup::parse(&input).map_err(|error| AppError::Usage(error.to_string()))?;

        let mode = if *replace_existing_projects {
            ImportMode::Replace
        } else if *skip_existing_projects {
            ImportMode::Skip
        } else {
            ImportMode::Fail
        };

        let plan = service.plan_import(&backup, mode).await?;
        let report = service.import(plan).await?;

        if output_json {
            let document = serde_json::json!({
                "projects": report.projects,
                "sections": report.sections,
                "tasks": report.tasks,
                "notes": report.notes,
            });
            output::print_json(&document)?;
        } else {
            println!(
                "Imported {} project(s), {} section(s), {} task(s), {} note(s)",
                report.projects, report.sections, report.tasks, report.notes
            );
        }
        return Ok(());
    }

    let project = resolve_project(&service, &config, cli.project.as_deref()).await?;

    let command = cli.command.expect("no subcommand is handled above");

    match command {
        Command::Project(project_cmd) => match project_cmd {
            ProjectCommand::List
            | ProjectCommand::Create { .. }
            | ProjectCommand::Show { .. }
            | ProjectCommand::Use { .. }
            | ProjectCommand::Delete { .. }
            | ProjectCommand::Current => {
                unreachable!("handled before the active project is resolved")
            }
            ProjectCommand::Update {
                name,
                description,
                sort,
                show_priority,
                hide_priority,
            } => {
                if name.is_none()
                    && description.is_none()
                    && sort.is_none()
                    && !show_priority
                    && !hide_priority
                {
                    return Err(AppError::Usage(
                        "No flags provided. Use 'scry project update --help' for available options."
                            .to_string(),
                    ));
                }
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
                        UpdateProject {
                            name,
                            description,
                            task_sorting_mode: sort,
                            show_priority,
                        },
                    )
                    .await?;
                if output_json {
                    output::print_json(&updated)?;
                } else {
                    println!("Updated project \"{}\".", updated.name);
                }
            }
        },
        Command::Section(section_cmd) => match section_cmd {
            SectionCommand::List => {
                let sections = service.list_sections_by_project_id(project.id).await?;
                if output_json {
                    output::print_json(&sections)?;
                } else {
                    println!("Sections for \"{}\":", project.name);
                    for section in &sections {
                        anstream::println!("  {}", color::status(section.color, &section.name));
                    }
                }
            }
            SectionCommand::Add { name, color } => {
                let section = service.create_section(&project, name, color).await?;
                if output_json {
                    output::print_json(&section)?;
                } else {
                    println!(
                        "Added section \"{}\" to project \"{}\"",
                        section.name, project.name
                    );
                }
            }
            SectionCommand::Remove { name } => {
                let section = service.get_section_by_name(project.id, &name).await?;
                let removed = service.delete_section(&project, section.id).await?;
                if output_json {
                    output::print_json(&removed)?;
                } else {
                    println!(
                        "Removed section \"{}\" from project \"{}\"; its tasks moved to No section",
                        name, project.name
                    );
                }
            }
            SectionCommand::Rename { old_name, new_name } => {
                let section = service.get_section_by_name(project.id, &old_name).await?;
                let renamed = service
                    .update_section(
                        section.id,
                        UpdateSection {
                            name: Some(new_name.clone()),
                            color: ColorChange::Keep,
                        },
                    )
                    .await?;
                if output_json {
                    output::print_json(&renamed)?;
                } else {
                    println!(
                        "Renamed section \"{}\" --> \"{}\" in project \"{}\"",
                        old_name, new_name, project.name
                    );
                }
            }
            SectionCommand::Merge { source, target } => {
                let source_section = service.get_section_by_name(project.id, &source).await?;
                let target_section = service.get_section_by_name(project.id, &target).await?;
                let (source_section, target_section) = service
                    .merge_sections(&project, source_section.id, target_section.id)
                    .await?;
                if output_json {
                    output::print_json(&target_section)?;
                } else {
                    println!(
                        "Merged section \"{}\" into \"{}\"",
                        source_section.name, target_section.name
                    );
                }
            }
            SectionCommand::SetColor { name, color } => {
                let section = service.get_section_by_name(project.id, &name).await?;
                let updated = service
                    .update_section(
                        section.id,
                        UpdateSection {
                            name: None,
                            color: ColorChange::Set(color),
                        },
                    )
                    .await?;
                if output_json {
                    output::print_json(&updated)?;
                } else {
                    println!(
                        "Set color of section \"{}\" to \"{}\" in project \"{}\"",
                        name, color, project.name
                    );
                }
            }
            SectionCommand::ResetColor { name } => {
                let section = service.get_section_by_name(project.id, &name).await?;
                let updated = service
                    .update_section(
                        section.id,
                        UpdateSection {
                            name: None,
                            color: ColorChange::Clear,
                        },
                    )
                    .await?;
                if output_json {
                    output::print_json(&updated)?;
                } else {
                    println!(
                        "Reset color of section \"{}\" in project \"{}\"",
                        name, project.name
                    );
                }
            }
        },
        Command::Add { .. }
        | Command::Update { .. }
        | Command::Move { .. }
        | Command::Duplicate { .. }
        | Command::Delete { .. }
        | Command::Close { .. }
        | Command::Open { .. }
        | Command::Note(_)
        | Command::Show { .. }
        | Command::List { .. } => {
            unreachable!("handled on the HTTP path")
        }
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
    service: &ProjectService,
    config: &ScryConfig,
    flag: Option<&str>,
) -> Result<Project, AppError> {
    if let Some(name) = flag {
        let project = service
            .get_project_by_name(name)
            .await?
            .ok_or_else(|| AppError::Usage(format!("project '{}' not found", name)))?;
        Ok(project)
    } else {
        let project = client_state::active_project(service, config).await?;
        Ok(project)
    }
}
