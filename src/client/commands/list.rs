//! `list`.
use super::{parse_when_arg, project_by_name};
use crate::client::{Client, ProjectDetail};
use crate::color;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::{print_json, section_label, task_line};
use crate::state::{DueFilter, ProjectState};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::ids::TaskId;
use scry_core::models::{ProjectSnapshot, Task, TaskSortingMode};
use std::collections::HashMap;

/// Filters and ordering for `scry list`.
pub struct ListOptions<'a> {
    pub section: Option<&'a str>,
    pub search: Option<&'a str>,
    pub include_closed: bool,
    pub overdue: bool,
    pub due_before: Option<&'a str>,
    pub due_after: Option<&'a str>,
    pub sort: TaskSortingMode,
}

/// List a project's tasks.
pub async fn list(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    options: ListOptions<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    let ListOptions {
        section,
        search,
        include_closed,
        overdue,
        due_before,
        due_after,
        sort,
    } = options;
    let project = project_by_name(client, config, project_flag).await?;

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
        comments: detail
            .tasks
            .iter()
            .flat_map(|task| task.comments.clone().unwrap_or_default())
            .collect(),
        automations: Vec::new(),
        events: Vec::new(),
    };

    let zone = TimeZone::system();
    let now = Timestamp::now();

    let mut state = ProjectState::from_snapshot(snapshot, sort, &zone);
    if let Some(search) = search.filter(|search| !search.is_empty()) {
        state = state.with_substring_filter(search.to_string());
    }

    let after = match due_after {
        Some(expr) => Some(
            parse_when_arg(expr, false)?
                .deadline(&zone)
                .map_err(|error| AppError::Usage(error.to_string()))?,
        ),
        None => None,
    };
    let before = match due_before {
        Some(expr) => Some(
            parse_when_arg(expr, false)?
                .deadline(&zone)
                .map_err(|error| AppError::Usage(error.to_string()))?,
        ),
        None => None,
    };

    let filter = DueFilter {
        overdue,
        after,
        before,
    };

    state = state.with_due_filter(filter, now, &zone);

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
            .flat_map(|group| group.tasks_with_comments.iter().map(Task::from))
            .collect();
        if include_closed {
            tasks.extend(state.closed_tasks().iter().map(Task::from));
        }
        print_json(&tasks)?;
        return Ok(());
    }

    println!("project \"{}\"\n", state.project().name);

    if groups.is_empty() && (!include_closed || state.closed_tasks().is_empty()) {
        println!("No tasks.");
        return Ok(());
    }

    for group in &groups {
        let header = format!(
            "{} ({}):",
            section_label(&group.section),
            group.tasks_with_comments.len()
        );
        anstream::println!(
            "{}",
            color::status(
                group.section.as_ref().and_then(|section| section.color),
                &header
            )
        );

        for task in &group.tasks_with_comments {
            anstream::println!("{}", task_line(task, &handles, now, &zone));
        }
    }

    if include_closed {
        println!("Closed ({}):", state.closed_tasks().len());
        for task in state.closed_tasks() {
            anstream::println!("{}", task_line(task, &handles, now, &zone));
        }
    }

    Ok(())
}
