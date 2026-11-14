//! Client-side replay of the confirmed replica and the unacknowledged outbox.
//!
//! The server is the only writer. A client keeps the last values it pulled as
//! confirmed state, applies new pull changes to it, then replays the operations
//! that are still in the outbox as plain field sets to get the visible state.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    Automation, AutomationAction, Change, ChangeKind, Color, Comment, EntityType, Priority,
    Project, ProjectSnapshot, Section, Tags, Task, TaskState,
};
use crate::position::Position;
use crate::sync::operations::{ColorChange, Operation, OperationKind};

/// A client's local replica: the projects it has confirmed, each with the full
/// project document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientState {
    pub projects: Vec<ProjectSnapshot>,
}

/// The confirmed state after applying a pull.
pub fn apply_changes(confirmed: &ClientState, changes: &[Change]) -> ClientState {
    let mut state = confirmed.clone();
    for change in changes {
        apply_change(&mut state, change);
    }
    state
}

/// The confirmed state with every unacknowledged outbox operation replayed on top.
pub fn replay_operations(confirmed: &ClientState, operations: &[Operation]) -> ClientState {
    let mut state = confirmed.clone();
    for operation in operations {
        apply_operation(&mut state, &operation.kind);
    }
    state
}

/// The visible state after a pull: overwrite confirmed values, drop acknowledged
/// operations from the outbox, then replay what remains.
pub fn visible_state(
    confirmed: &ClientState,
    changes: &[Change],
    acknowledged: &[Uuid],
    outbox: &[Operation],
) -> ClientState {
    let confirmed = apply_changes(confirmed, changes);
    let acknowledged: HashSet<Uuid> = acknowledged.iter().copied().collect();
    let remaining: Vec<Operation> = outbox
        .iter()
        .filter(|operation| !acknowledged.contains(&operation.operation_id))
        .cloned()
        .collect();

    replay_operations(&confirmed, &remaining)
}

fn apply_operation(state: &mut ClientState, kind: &OperationKind) {
    match kind {
        OperationKind::CreateProject {
            id,
            name,
            description,
            created_at,
            updated_at,
        } => {
            let project = Project {
                id: *id,
                name: name.clone(),
                description: description.clone(),
                created_at: *created_at,
                updated_at: (*updated_at).unwrap_or(*created_at),
            };
            if let Some(snapshot) = state.projects.iter_mut().find(|p| p.project.id == *id) {
                snapshot.project = project;
            } else {
                state.projects.push(snapshot(project));
            }
        }
        OperationKind::UpdateProject {
            id,
            name,
            description,
            updated_at,
        } => {
            if let Some(snapshot) = state.projects.iter_mut().find(|p| p.project.id == *id) {
                let project = &mut snapshot.project;
                if let Some(name) = name {
                    project.name = name.clone();
                }
                if let Some(description) = description {
                    project.description = description.clone();
                }
                if let Some(updated_at) = updated_at {
                    project.updated_at = *updated_at;
                }
            }
        }
        OperationKind::DeleteProject { id } => {
            state.projects.retain(|snapshot| snapshot.project.id != *id);
        }
        OperationKind::CreateSection {
            id,
            project_id,
            name,
            position,
            color,
            created_at,
            updated_at,
        } => {
            let section = Section {
                id: *id,
                project_id: *project_id,
                name: name.clone(),
                position: position.clone(),
                color: *color,
                created_at: *created_at,
                updated_at: (*updated_at).unwrap_or(*created_at),
            };
            if let Some(snapshot) = state
                .projects
                .iter_mut()
                .find(|p| p.project.id == *project_id)
            {
                upsert_section(snapshot, section);
            }
        }
        OperationKind::UpdateSection {
            id,
            name,
            color,
            updated_at,
        } => {
            if let Some(section) = find_section_mut(state, *id) {
                if let Some(name) = name {
                    section.name = name.clone();
                }
                if let Some(color) = color {
                    section.color = match color {
                        ColorChange::Clear => None,
                        ColorChange::Set(color) => Some(*color),
                    };
                }
                if let Some(updated_at) = updated_at {
                    section.updated_at = *updated_at;
                }
            }
        }
        OperationKind::MoveSection {
            id,
            position,
            updated_at,
        } => {
            if let Some(section) = find_section_mut(state, *id) {
                section.position = position.clone();
                if let Some(updated_at) = updated_at {
                    section.updated_at = *updated_at;
                }
            }
        }
        OperationKind::DeleteSection { id } => {
            remove_section(state, *id);
        }
        OperationKind::CreateTask {
            id,
            project_id,
            title,
            description,
            priority,
            section_id,
            position,
            state: task_state,
            tags,
            created_at,
            updated_at,
            external_identity,
            external_metadata,
        } => {
            let task = Task {
                id: *id,
                project_id: *project_id,
                title: title.clone(),
                description: description.clone(),
                priority: *priority,
                section_id: *section_id,
                position: position.clone(),
                state: *task_state,
                tags: tags.clone(),
                created_at: *created_at,
                updated_at: (*updated_at).unwrap_or(*created_at),
                external_identity: external_identity.clone(),
                external_metadata: external_metadata.as_deref().cloned(),
            };
            if let Some(snapshot) = state
                .projects
                .iter_mut()
                .find(|p| p.project.id == *project_id)
            {
                upsert_task(snapshot, task);
            }
        }
        OperationKind::UpdateTask {
            id,
            title,
            description,
            priority,
            tags,
            state: task_state,
            external_identity,
            external_metadata,
            updated_at,
        } => {
            if let Some(task) = find_task_mut(state, *id) {
                if let Some(title) = title {
                    task.title = title.clone();
                }
                if let Some(description) = description {
                    task.description = description.clone();
                }
                if let Some(priority) = priority {
                    task.priority = *priority;
                }
                if let Some(tags) = tags {
                    task.tags = tags.clone();
                }
                if let Some(task_state) = task_state {
                    task.state = *task_state;
                }
                if let Some(identity) = external_identity {
                    task.external_identity = identity.clone();
                }
                if let Some(metadata) = external_metadata {
                    task.external_metadata = metadata.as_deref().cloned();
                }
                if let Some(updated_at) = updated_at {
                    task.updated_at = *updated_at;
                }
            }
        }
        OperationKind::MoveTask {
            id,
            section_id,
            position,
            updated_at,
        } => {
            if let Some(task) = find_task_mut(state, *id) {
                task.section_id = *section_id;
                task.position = position.clone();
                if let Some(updated_at) = updated_at {
                    task.updated_at = *updated_at;
                }
            }
        }
        OperationKind::DeleteTask { id } => {
            remove_task(state, *id);
        }
        OperationKind::CreateComment {
            id,
            task_id,
            contents,
            created_at,
            updated_at,
        } => {
            let comment = Comment {
                id: *id,
                task_id: *task_id,
                contents: contents.clone(),
                created_at: *created_at,
                updated_at: (*updated_at).unwrap_or(*created_at),
            };
            if let Some(snapshot) = state
                .projects
                .iter_mut()
                .find(|p| p.tasks.iter().any(|task| task.id == *task_id))
            {
                upsert_comment(snapshot, comment);
            }
        }
        OperationKind::UpdateComment {
            id,
            contents,
            updated_at,
        } => {
            if let Some(comment) = find_comment_mut(state, *id) {
                comment.contents = contents.clone();
                if let Some(updated_at) = updated_at {
                    comment.updated_at = *updated_at;
                }
            }
        }
        OperationKind::DeleteComment { id } => {
            remove_comment(state, *id);
        }
        OperationKind::CreateAutomation {
            id,
            project_id,
            trigger,
            actions,
            created_at,
            updated_at,
        } => {
            let automation = Automation {
                id: *id,
                project_id: *project_id,
                trigger: trigger.clone(),
                actions: actions.clone(),
                created_at: *created_at,
                updated_at: (*updated_at).unwrap_or(*created_at),
            };
            if let Some(snapshot) = state
                .projects
                .iter_mut()
                .find(|p| p.project.id == *project_id)
            {
                upsert_automation(snapshot, automation);
            }
        }
        OperationKind::UpdateAutomation {
            id,
            actions,
            updated_at,
        } => {
            if let Some(automation) = find_automation_mut(state, *id) {
                automation.actions = actions.clone();
                if let Some(updated_at) = updated_at {
                    automation.updated_at = *updated_at;
                }
            }
        }
        OperationKind::DeleteAutomation { id } => {
            remove_automation(state, *id);
        }
    }
}

fn apply_change(state: &mut ClientState, change: &Change) {
    match (change.entity_type, change.kind) {
        (EntityType::Project, ChangeKind::Create) => {
            if let Ok(project) = serde_json::from_value::<Project>(change.field_values.clone()) {
                if let Some(existing) = state
                    .projects
                    .iter_mut()
                    .find(|snapshot| snapshot.project.id == project.id)
                {
                    existing.project = project;
                } else {
                    state.projects.push(snapshot(project));
                }
            }
        }
        (EntityType::Project, ChangeKind::Update) => {
            if let Some(existing) = state
                .projects
                .iter_mut()
                .find(|snapshot| snapshot.project.id == change.entity_id)
            {
                apply_project_values(&mut existing.project, &change.field_values);
            }
        }
        (EntityType::Project, ChangeKind::Delete) => {
            state.projects.retain(|p| p.project.id != change.entity_id);
        }
        (EntityType::Section, ChangeKind::Create) => {
            if let Ok(section) = serde_json::from_value::<Section>(change.field_values.clone())
                && let Some(existing) = state
                    .projects
                    .iter_mut()
                    .find(|snapshot| snapshot.project.id == section.project_id)
            {
                upsert_section(existing, section);
            }
        }
        (EntityType::Section, ChangeKind::Update) => {
            if let Some(section) = find_section_mut(state, change.entity_id) {
                apply_section_values(section, &change.field_values);
            }
        }
        (EntityType::Section, ChangeKind::Delete) => {
            remove_section(state, change.entity_id);
        }
        (EntityType::Task, ChangeKind::Create) => {
            if let Ok(task) = serde_json::from_value::<Task>(change.field_values.clone())
                && let Some(existing) = state
                    .projects
                    .iter_mut()
                    .find(|snapshot| snapshot.project.id == task.project_id)
            {
                upsert_task(existing, task);
            }
        }
        (EntityType::Task, ChangeKind::Update) => {
            if let Some(task) = find_task_mut(state, change.entity_id) {
                apply_task_values(task, &change.field_values);
            }
        }
        (EntityType::Task, ChangeKind::Delete) => {
            remove_task(state, change.entity_id);
        }
        (EntityType::Comment, ChangeKind::Create) => {
            if let Ok(comment) = serde_json::from_value::<Comment>(change.field_values.clone())
                && let Some(existing) = state
                    .projects
                    .iter_mut()
                    .find(|snapshot| snapshot.tasks.iter().any(|task| task.id == comment.task_id))
            {
                upsert_comment(existing, comment);
            }
        }
        (EntityType::Comment, ChangeKind::Update) => {
            if let Some(comment) = find_comment_mut(state, change.entity_id) {
                if let Some(contents) = change.field_values.get("contents").and_then(|v| v.as_str())
                {
                    comment.contents = contents.to_string();
                }
                apply_updated_at(&mut comment.updated_at, &change.field_values);
            }
        }
        (EntityType::Comment, ChangeKind::Delete) => {
            remove_comment(state, change.entity_id);
        }
        (EntityType::Automation, ChangeKind::Create) => {
            if let Ok(automation) =
                serde_json::from_value::<Automation>(change.field_values.clone())
                && let Some(existing) = state
                    .projects
                    .iter_mut()
                    .find(|snapshot| snapshot.project.id == automation.project_id)
            {
                upsert_automation(existing, automation);
            }
        }
        (EntityType::Automation, ChangeKind::Update) => {
            if let Some(automation) = find_automation_mut(state, change.entity_id) {
                if let Some(actions) = change.field_values.get("actions")
                    && let Ok(actions) =
                        serde_json::from_value::<Vec<AutomationAction>>(actions.clone())
                {
                    automation.actions = actions;
                }
                apply_updated_at(&mut automation.updated_at, &change.field_values);
            }
        }
        (EntityType::Automation, ChangeKind::Delete) => {
            remove_automation(state, change.entity_id);
        }
    }
}

/// Apply the `updated_at` carried by a change payload, when present.
fn apply_updated_at(updated_at: &mut DateTime<Utc>, values: &serde_json::Value) {
    if let Some(value) = values.get("updated_at")
        && let Ok(parsed) = serde_json::from_value::<DateTime<Utc>>(value.clone())
    {
        *updated_at = parsed;
    }
}

fn apply_project_values(project: &mut Project, values: &serde_json::Value) {
    if let Some(name) = values.get("name").and_then(|value| value.as_str()) {
        project.name = name.to_string();
    }
    if let Some(description) = values.get("description") {
        project.description = description.as_str().map(str::to_string);
    }
    apply_updated_at(&mut project.updated_at, values);
}

fn apply_section_values(section: &mut Section, values: &serde_json::Value) {
    if let Some(name) = values.get("name").and_then(|value| value.as_str()) {
        section.name = name.to_string();
    }
    if let Some(color) = values.get("color") {
        section.color = if color.is_null() {
            None
        } else {
            serde_json::from_value::<Color>(color.clone()).ok()
        };
    }
    if let Some(position) = values.get("position")
        && let Ok(position) = serde_json::from_value::<Position>(position.clone())
    {
        section.position = position;
    }
    apply_updated_at(&mut section.updated_at, values);
}

fn apply_task_values(task: &mut Task, values: &serde_json::Value) {
    if let Some(title) = values.get("title").and_then(|value| value.as_str()) {
        task.title = title.to_string();
    }
    if let Some(description) = values.get("description") {
        task.description = description.as_str().map(str::to_string);
    }
    if let Some(priority) = values.get("priority")
        && let Ok(priority) = serde_json::from_value::<Priority>(priority.clone())
    {
        task.priority = priority;
    }
    if let Some(tags) = values.get("tags")
        && let Ok(tags) = serde_json::from_value::<Tags>(tags.clone())
    {
        task.tags = tags;
    }
    if let Some(state) = values.get("state")
        && let Ok(state) = serde_json::from_value::<TaskState>(state.clone())
    {
        task.state = state;
    }
    if let Some(identity) = values.get("external_identity") {
        task.external_identity = serde_json::from_value(identity.clone()).ok();
    }
    if let Some(metadata) = values.get("external_metadata") {
        task.external_metadata = serde_json::from_value(metadata.clone()).ok();
    }
    if let Some(section_id) = values.get("section_id") {
        task.section_id = section_id
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok());
    }
    if let Some(position) = values.get("position")
        && let Ok(position) = serde_json::from_value::<Position>(position.clone())
    {
        task.position = position;
    }
    apply_updated_at(&mut task.updated_at, values);
}

fn snapshot(project: Project) -> ProjectSnapshot {
    ProjectSnapshot {
        project,
        sections: Vec::new(),
        tasks: Vec::new(),
        comments: Vec::new(),
        automations: Vec::new(),
    }
}

fn upsert_section(snapshot: &mut ProjectSnapshot, section: Section) {
    if let Some(existing) = snapshot
        .sections
        .iter_mut()
        .find(|existing| existing.id == section.id)
    {
        *existing = section;
    } else {
        snapshot.sections.push(section);
    }
}

fn upsert_task(snapshot: &mut ProjectSnapshot, task: Task) {
    if let Some(existing) = snapshot
        .tasks
        .iter_mut()
        .find(|existing| existing.id == task.id)
    {
        *existing = task;
    } else {
        snapshot.tasks.push(task);
    }
}

fn upsert_comment(snapshot: &mut ProjectSnapshot, comment: Comment) {
    if let Some(existing) = snapshot
        .comments
        .iter_mut()
        .find(|existing| existing.id == comment.id)
    {
        *existing = comment;
    } else {
        snapshot.comments.push(comment);
    }
}

fn upsert_automation(snapshot: &mut ProjectSnapshot, automation: Automation) {
    if let Some(existing) = snapshot
        .automations
        .iter_mut()
        .find(|existing| existing.id == automation.id)
    {
        *existing = automation;
    } else {
        snapshot.automations.push(automation);
    }
}

fn find_section_mut(state: &mut ClientState, id: Uuid) -> Option<&mut Section> {
    state.projects.iter_mut().find_map(|snapshot| {
        snapshot
            .sections
            .iter_mut()
            .find(|section| section.id == id)
    })
}

fn find_task_mut(state: &mut ClientState, id: Uuid) -> Option<&mut Task> {
    state
        .projects
        .iter_mut()
        .find_map(|snapshot| snapshot.tasks.iter_mut().find(|task| task.id == id))
}

fn find_comment_mut(state: &mut ClientState, id: Uuid) -> Option<&mut Comment> {
    state.projects.iter_mut().find_map(|snapshot| {
        snapshot
            .comments
            .iter_mut()
            .find(|comment| comment.id == id)
    })
}

fn find_automation_mut(state: &mut ClientState, id: Uuid) -> Option<&mut Automation> {
    state.projects.iter_mut().find_map(|snapshot| {
        snapshot
            .automations
            .iter_mut()
            .find(|automation| automation.id == id)
    })
}

fn remove_section(state: &mut ClientState, id: Uuid) {
    for snapshot in &mut state.projects {
        if let Some(index) = snapshot
            .sections
            .iter()
            .position(|section| section.id == id)
        {
            snapshot.sections.remove(index);
            for task in &mut snapshot.tasks {
                if task.section_id == Some(id) {
                    task.section_id = None;
                }
            }
            return;
        }
    }
}

fn remove_task(state: &mut ClientState, id: Uuid) {
    for snapshot in &mut state.projects {
        if let Some(index) = snapshot.tasks.iter().position(|task| task.id == id) {
            snapshot.tasks.remove(index);
            snapshot.comments.retain(|comment| comment.task_id != id);
            return;
        }
    }
}

fn remove_comment(state: &mut ClientState, id: Uuid) {
    for snapshot in &mut state.projects {
        snapshot.comments.retain(|comment| comment.id != id);
    }
}

fn remove_automation(state: &mut ClientState, id: Uuid) {
    for snapshot in &mut state.projects {
        snapshot
            .automations
            .retain(|automation| automation.id != id);
    }
}
