//! Client command implementations.

use std::collections::HashMap;

use chrono::Local;
use scry_core::ids::TaskId;
use scry_core::models::{Lifecycle, Project, Section, Task};

use crate::client::{Client, ProjectDetail, TaskDetails};
use crate::client_state::ClientState;
use crate::color;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::{print_json, section_label, task_line};
use crate::state::{DATETIME_FORMAT_STR, ProjectState};
use crate::store::ProjectSnapshot;

/// List all projects.
pub async fn project_list(
    client: &Client,
    config: &ScryConfig,
    output_json: bool,
) -> Result<(), AppError> {
    let projects: Vec<Project> = client.get_json(&["projects"], &[]).await?;

    if output_json {
        print_json(&projects)?;
    } else if projects.is_empty() {
        eprintln!("No projects. Run 'scry project create <name>' to create one.");
    } else {
        let active = ClientState::load(config)?
            .active_project
            .unwrap_or_else(|| "default".to_string());

        for project in &projects {
            let marker = if project.name == active { "* " } else { "  " };
            println!("  {}{}", marker, project.name);
        }
    }

    Ok(())
}

/// List a project's tasks.
pub async fn list(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    section: Option<&str>,
    search: Option<&str>,
    include_closed: bool,
    output_json: bool,
) -> Result<(), AppError> {
    let project_name = match project_flag {
        Some(name) => name.to_string(),
        None => ClientState::load(config)?
            .active_project
            .unwrap_or_else(|| "default".to_string()),
    };

    let project: Option<Project> = client
        .get_json(&["projects", "by-name", project_name.as_str()], &[])
        .await?;

    let project = project.ok_or_else(|| AppError::Remote {
        kind: "not_found",
        message: format!("project '{project_name}' not found"),
        details: None,
    })?;

    let project_id = project.id.to_string();
    let detail: ProjectDetail = client
        .get_json(
            &["projects", project_id.as_str()],
            &[("detailed", "true"), ("include_handles", "true")],
        )
        .await?;

    let selected_section_id = match section {
        Some(name) => Some(
            detail
                .sections
                .iter()
                .find(|candidate| candidate.name == name)
                .map(|candidate| candidate.id)
                .ok_or_else(|| AppError::Remote {
                    kind: "not_found",
                    message: format!("section '{name}' not found"),
                    details: None,
                })?,
        ),
        None => None,
    };

    let handles: HashMap<TaskId, String> = detail
        .tasks
        .iter()
        .filter_map(|task| task.handle.clone().map(|handle| (task.task.id, handle)))
        .collect();

    let snapshot = ProjectSnapshot {
        project: detail.project,
        sections: detail.sections,
        tasks: detail.tasks.iter().map(|task| task.task.clone()).collect(),
        notes: detail
            .tasks
            .iter()
            .flat_map(|task| task.notes.clone().unwrap_or_default())
            .collect(),
    };

    let mut state = ProjectState::from_snapshot(snapshot);
    if let Some(search) = search.filter(|search| !search.is_empty()) {
        state = state.with_substring_filter(search.to_string());
    }

    let groups: Vec<_> = state
        .section_groups()
        .filter(|group| match selected_section_id {
            Some(section_id) => group
                .section
                .as_ref()
                .is_some_and(|section| section.id == section_id),
            None => true,
        })
        .collect();

    if output_json {
        let mut tasks: Vec<Task> = groups
            .iter()
            .flat_map(|group| group.tasks_with_notes.iter().map(Task::from))
            .collect();
        if include_closed {
            tasks.extend(state.closed_tasks().iter().map(Task::from));
        }
        print_json(&tasks)?;
        return Ok(());
    }

    println!("project \"{}\"\n", state.project().name);

    let show_priority = state.project().show_priority;

    if groups.is_empty() && (!include_closed || state.closed_tasks().is_empty()) {
        println!("No tasks.");
        return Ok(());
    }

    for group in &groups {
        let header = format!(
            "{} ({}):",
            section_label(&group.section),
            group.tasks_with_notes.len()
        );
        anstream::println!(
            "{}",
            color::status(
                group.section.as_ref().and_then(|section| section.color),
                &header
            )
        );

        for task in &group.tasks_with_notes {
            anstream::println!("{}", task_line(task, show_priority, &handles));
        }
    }

    if include_closed {
        println!("Closed ({}):", state.closed_tasks().len());
        for task in state.closed_tasks() {
            anstream::println!("{}", task_line(task, show_priority, &handles));
        }
    }

    Ok(())
}

/// Show one task with its notes.
pub async fn show(client: &Client, handle: &str, output_json: bool) -> Result<(), AppError> {
    let detail: TaskDetails = client
        .get_json(
            &["tasks", handle],
            &[("detailed", "true"), ("include_handle", "true")],
        )
        .await?;

    if output_json {
        print_json(&detail)?;
        return Ok(());
    }

    let section: Option<Section> = match detail.task.section_id {
        Some(section_id) => {
            let section_id = section_id.to_string();
            client
                .get_json(&["sections", section_id.as_str()], &[])
                .await?
        }
        None => None,
    };

    let task = &detail.task;
    let display_handle = detail.handle.as_deref().unwrap_or(handle);

    let state_label = match &task.lifecycle {
        Lifecycle::Open => "open".to_string(),
        Lifecycle::Closed { closed_at, reason } => format!(
            "{} {}",
            reason.as_str(),
            closed_at.with_timezone(&Local).format(DATETIME_FORMAT_STR)
        ),
    };

    println!("{} #{}", task.title, display_handle);
    println!();

    if let Some(description) = &task.description {
        for line in description.lines() {
            println!("    {}", line);
        }
    }
    println!();

    anstream::println!("Priority:    {}", color::priority_long(task.priority));
    anstream::println!(
        "Section:     {}",
        color::status(
            section.as_ref().and_then(|section| section.color),
            section_label(&section)
        )
    );
    anstream::println!("State:       {}", state_label);
    let tags = task
        .tags
        .iter()
        .map(|tag| color::tag(tag))
        .collect::<Vec<_>>()
        .join(" ");
    anstream::println!("Tags:        {}", tags);
    println!(
        "Created at:  {}",
        task.created_at
            .with_timezone(&Local)
            .format(DATETIME_FORMAT_STR)
    );
    println!();

    for note in detail.notes.unwrap_or_default() {
        let note_created_at = note
            .created_at
            .with_timezone(&Local)
            .format(DATETIME_FORMAT_STR);
        println!("{}", note_created_at);
        println!("{}", note.contents);
        println!();
    }

    Ok(())
}
