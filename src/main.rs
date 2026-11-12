mod client;
mod client_state;
mod color;
pub use scry_core::config;
mod docs;
mod error;
mod output;
mod skill;
mod state;

use crate::client::{AutomationActionChoice, AutomationTriggerKind};
use crate::color::ColorChoice;
use crate::skill::Harness;
use clap::{CommandFactory, Parser, Subcommand};
use config::{ScryConfig, ScrySecrets};
use error::AppError;
use scry_core::ids::Handle;
use scry_core::models::{Color, Priority, TaskSortingMode};
use scry_core::snapshot::ImportMode;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "scry",
    about = "A task manager for the terminal and the browser",
    version
)]
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
    #[command(group(
        clap::ArgGroup::new("task_source")
            .required(true)
            .multiple(true)
            .args(["title", "external_url"])
    ))]
    Add {
        /// The task title (optional when --external-url is provided)
        title: Option<String>,
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
        /// Link the task to a GitHub issue or pull request
        #[arg(long, value_name = "URL")]
        external_url: Option<String>,
    },
    /// Move a task to a section or place it within its group
    #[command(group(
        clap::ArgGroup::new("placement")
            .multiple(false)
            .args(["top", "bottom", "before", "after"])
    ))]
    Move {
        /// The task ID or a unique handle prefix
        id: Handle,
        /// The target section
        section: Option<String>,
        /// Move the task to No section
        #[arg(long, conflicts_with = "section")]
        no_section: bool,
        /// Place the task first in its destination
        #[arg(long, visible_alias = "first")]
        top: bool,
        /// Place the task last in its destination
        #[arg(long, visible_alias = "last")]
        bottom: bool,
        /// Place the task immediately before this task
        #[arg(long, value_name = "HANDLE")]
        before: Option<String>,
        /// Place the task immediately after this task
        #[arg(long, value_name = "HANDLE")]
        after: Option<String>,
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
        /// Link or relink the task to a GitHub issue or pull request
        #[arg(long, value_name = "URL", conflicts_with = "clear_external_url")]
        external_url: Option<String>,
        /// Remove the external identity and metadata
        #[arg(long, conflicts_with = "external_url")]
        clear_external_url: bool,
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
    /// Close tasks
    Close {
        /// The task IDs or unique handle prefixes
        #[arg(required = true, num_args = 1..)]
        ids: Vec<Handle>,
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
    /// Snapshot all data (or a single project with the global -p/--project) as JSON to stdout
    Export,
    /// Import a JSON snapshot from stdin
    Import {
        /// Replace the existing project when the snapshot has a project with the same name
        #[arg(long, conflicts_with = "skip_existing_projects")]
        replace_existing_projects: bool,
        /// Keep the existing project and skip the snapshot's project of the same name
        #[arg(long, conflicts_with = "replace_existing_projects")]
        skip_existing_projects: bool,
    },
    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Manage sections in the active or -p project
    #[command(subcommand)]
    Section(SectionCommand),
    /// Manage automations in the active or -p project
    #[command(subcommand)]
    Automation(AutomationCommand),
    /// Print the bundled documentation (the README) to stdout
    Docs,
    /// Manage the agent skill that teaches coding agents to drive scry
    #[command(subcommand)]
    Skill(SkillCommand),
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
    /// Move a section to the top, bottom, or next to another section
    #[command(group(
        clap::ArgGroup::new("placement")
            .required(true)
            .multiple(false)
            .args(["top", "bottom", "before", "after"])
    ))]
    Move {
        /// The section name
        name: String,
        /// Place the section first
        #[arg(long, visible_alias = "first")]
        top: bool,
        /// Place the section last
        #[arg(long, visible_alias = "last")]
        bottom: bool,
        /// Place the section immediately before this section
        #[arg(long, value_name = "NAME")]
        before: Option<String>,
        /// Place the section immediately after this section
        #[arg(long, value_name = "NAME")]
        after: Option<String>,
    },
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
enum AutomationCommand {
    /// List automations
    List,
    /// Create or update the action for a trigger
    #[command(group(
        clap::ArgGroup::new("action")
            .required(true)
            .multiple(false)
            .args(["move_to", "no_section", "close", "open", "priority"])
    ))]
    Set {
        /// The trigger: created, closed, opened, or enters
        #[arg(long, value_enum)]
        when: AutomationTriggerKind,
        /// The section for --when enters
        #[arg(long, value_name = "SECTION")]
        section: Option<String>,
        /// Move the task to this section
        #[arg(long, value_name = "SECTION")]
        move_to: Option<String>,
        /// Move the task to No section
        #[arg(long)]
        no_section: bool,
        /// Close the task
        #[arg(long)]
        close: bool,
        /// Open the task
        #[arg(long)]
        open: bool,
        /// Set the task priority
        #[arg(long, value_enum)]
        priority: Option<Priority>,
    },
    /// Remove the automation for a trigger
    Remove {
        /// The trigger: created, closed, opened, or enters
        #[arg(long, value_enum)]
        when: AutomationTriggerKind,
        /// The section for --when enters
        #[arg(long, value_name = "SECTION")]
        section: Option<String>,
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
    let mut document = serde_json::json!({
        "error": {
            "kind": error.kind(),
            "message": error.to_string(),
        }
    });

    if let Some(details) = error.details() {
        document["error"]["details"] = details;
    }

    document.to_string()
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let output_json = cli.json;
    if output_json
        && (cli.command.is_none()
            || matches!(
                cli.command.as_ref(),
                Some(Command::Docs | Command::Skill(_))
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
            external_url,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::add(
                &client,
                &config,
                client::AddTask {
                    project: cli.project.as_deref(),
                    title: title.as_deref(),
                    description: description.as_deref(),
                    priority: *priority,
                    tags: tags.as_deref(),
                    section: section.as_deref(),
                    new_section: new_section.as_deref(),
                    external_url: external_url.as_deref(),
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
            external_url,
            clear_external_url,
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
                    external_url: external_url.as_deref(),
                    clear_external_url: *clear_external_url,
                },
                output_json,
            )
            .await;
        }
        Some(Command::Move {
            id,
            section,
            no_section,
            top,
            bottom,
            before,
            after,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            let placement = if *top {
                Some(client::TaskMovePlacement::Top)
            } else if *bottom {
                Some(client::TaskMovePlacement::Bottom)
            } else if let Some(anchor) = before.as_deref() {
                Some(client::TaskMovePlacement::Before(anchor))
            } else {
                after.as_deref().map(client::TaskMovePlacement::After)
            };
            return client::move_task(
                &client,
                client::MoveTask {
                    handle: id.as_str(),
                    section: section.as_deref(),
                    no_section: *no_section,
                    placement,
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
        Some(Command::Close { ids }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::close_tasks(&client, ids, output_json).await;
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
        Some(Command::Project(ProjectCommand::Create {
            name,
            template_name,
            description,
        })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_create(
                &client,
                client::CreateProject {
                    name,
                    description: description.as_deref(),
                    template: template_name.as_deref(),
                },
                output_json,
            )
            .await;
        }
        Some(Command::Project(ProjectCommand::Update {
            name,
            description,
            sort,
            show_priority,
            hide_priority,
        })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            let show_priority = if *show_priority {
                Some(true)
            } else if *hide_priority {
                Some(false)
            } else {
                None
            };

            return client::project_update(
                &client,
                &config,
                cli.project.as_deref(),
                client::UpdateProject {
                    name: name.as_deref(),
                    description: description.as_deref(),
                    sort: *sort,
                    show_priority,
                },
                output_json,
            )
            .await;
        }
        Some(Command::Project(ProjectCommand::Delete { name, force })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::project_delete(&client, name, *force, output_json).await;
        }
        Some(Command::Section(SectionCommand::List)) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_list(&client, &config, cli.project.as_deref(), output_json)
                .await;
        }
        Some(Command::Section(SectionCommand::Move {
            name,
            top,
            bottom,
            before,
            after,
        })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            let placement = if *top {
                client::SectionMovePlacement::Top
            } else if *bottom {
                client::SectionMovePlacement::Bottom
            } else if let Some(anchor) = before.as_deref() {
                client::SectionMovePlacement::Before(anchor)
            } else if let Some(anchor) = after.as_deref() {
                client::SectionMovePlacement::After(anchor)
            } else {
                return Err(AppError::Usage(
                    "Specify one of --top, --bottom, --before, or --after.".to_string(),
                ));
            };
            return client::section_move(
                &client,
                &config,
                cli.project.as_deref(),
                name,
                placement,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::Add { name, color })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_add(
                &client,
                &config,
                cli.project.as_deref(),
                name,
                *color,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::Remove { name })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_remove(
                &client,
                &config,
                cli.project.as_deref(),
                name,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::Rename { old_name, new_name })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_rename(
                &client,
                &config,
                cli.project.as_deref(),
                old_name,
                new_name,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::Merge { source, target })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_merge(
                &client,
                &config,
                cli.project.as_deref(),
                source,
                target,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::SetColor { name, color })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_set_color(
                &client,
                &config,
                cli.project.as_deref(),
                name,
                *color,
                output_json,
            )
            .await;
        }
        Some(Command::Section(SectionCommand::ResetColor { name })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::section_reset_color(
                &client,
                &config,
                cli.project.as_deref(),
                name,
                output_json,
            )
            .await;
        }
        Some(Command::Automation(AutomationCommand::List)) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::automation_list(&client, &config, cli.project.as_deref(), output_json)
                .await;
        }
        Some(Command::Automation(AutomationCommand::Set {
            when,
            section,
            move_to,
            no_section,
            close,
            open,
            priority,
        })) => {
            let action = if let Some(target) = move_to {
                AutomationActionChoice::MoveTo(target.clone())
            } else if *no_section {
                AutomationActionChoice::NoSection
            } else if *close {
                AutomationActionChoice::Close
            } else if *open {
                AutomationActionChoice::Open
            } else if let Some(priority) = priority {
                AutomationActionChoice::Priority(*priority)
            } else {
                return Err(AppError::Usage(
                    "Specify one automation action.".to_string(),
                ));
            };

            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::automation_set(
                &client,
                &config,
                cli.project.as_deref(),
                *when,
                section.as_deref(),
                action,
                output_json,
            )
            .await;
        }
        Some(Command::Automation(AutomationCommand::Remove { when, section })) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::automation_remove(
                &client,
                &config,
                cli.project.as_deref(),
                *when,
                section.as_deref(),
                output_json,
            )
            .await;
        }
        Some(Command::Export) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            return client::export(&client, &config, cli.project.as_deref()).await;
        }
        Some(Command::Import {
            replace_existing_projects,
            skip_existing_projects,
        }) => {
            let client = client::Client::new(&config, cli.server.as_deref(), &secrets)?;
            let mode = if *replace_existing_projects {
                ImportMode::Replace
            } else if *skip_existing_projects {
                ImportMode::Skip
            } else {
                ImportMode::Merge
            };

            return client::import(&client, mode, output_json).await;
        }
        _ => {}
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
