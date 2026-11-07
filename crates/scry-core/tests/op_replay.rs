use scry_core::models::{Change, Priority, Task, TaskState};
use scry_core::sync::operations::Operation;
use scry_core::sync::replay::{ClientState, apply_changes, replay_operations, visible_state};
use serde_json::json;
use uuid::Uuid;

const PROJECT_ID: &str = "11111111-1111-4111-8111-111111111111";
const SECTION_ID: &str = "22222222-2222-4222-8222-222222222222";
const TASK_ID: &str = "44444444-4444-4444-8444-444444444444";
const NOTE_ID: &str = "66666666-6666-4666-8666-666666666666";
const OPERATION_ID: &str = "c0000000-0000-4000-8000-000000000001";
const NOW: &str = "2026-01-01T00:00:00Z";

fn confirmed() -> ClientState {
    serde_json::from_value(json!({
        "projects": [{
            "id": PROJECT_ID,
            "name": "alpha",
            "description": null,
            "task_sorting_mode": "manual",
            "show_priority": true,
            "created_at": NOW,
            "sections": [{
                "id": SECTION_ID,
                "project_id": PROJECT_ID,
                "name": "backlog",
                "position": "a0",
                "color": null,
                "created_at": NOW
            }],
            "tasks": [{
                "id": TASK_ID,
                "project_id": PROJECT_ID,
                "title": "first",
                "description": null,
                "priority": "medium",
                "section_id": SECTION_ID,
                "position": "a0",
                "state": "open",
                "tags": ["one"],
                "created_at": NOW
            }],
            "notes": [{
                "id": NOTE_ID,
                "task_id": TASK_ID,
                "contents": "note one",
                "created_at": NOW
            }]
        }]
    }))
    .unwrap()
}

fn update_task_operation() -> Operation {
    serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "update_task",
        "id": TASK_ID,
        "title": "local",
        "priority": "high",
        "state": "closed"
    }))
    .unwrap()
}

fn server_title_change(title: &str) -> Change {
    serde_json::from_value(json!({
        "seq": 2,
        "entity_type": "task",
        "entity_id": TASK_ID,
        "kind": "update",
        "operation_id": OPERATION_ID,
        "client_timestamp": null,
        "field_values": { "title": title },
        "created_at": NOW
    }))
    .unwrap()
}

fn server_section_position_change(position: &str) -> Change {
    serde_json::from_value(json!({
        "seq": 2,
        "entity_type": "section",
        "entity_id": SECTION_ID,
        "kind": "update",
        "operation_id": OPERATION_ID,
        "client_timestamp": null,
        "field_values": { "position": position },
        "created_at": NOW
    }))
    .unwrap()
}

fn move_section_operation(position: &str) -> Operation {
    serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "move_section",
        "id": SECTION_ID,
        "position": position
    }))
    .unwrap()
}

fn only_task(state: &ClientState) -> &Task {
    &state.projects[0].tasks[0]
}

#[test]
fn replay_operations_applies_update_task() {
    let confirmed = confirmed();
    let state = replay_operations(&confirmed, &[update_task_operation()]);
    let task = only_task(&state);

    assert_eq!(task.title, "local");
    assert_eq!(task.priority, Priority::High);
    assert_eq!(task.state, TaskState::Closed);
    assert_eq!(state.projects[0].notes[0].contents, "note one");
    assert_eq!(only_task(&confirmed).title, "first");
}

#[test]
fn apply_changes_applies_update_task() {
    let state = apply_changes(&confirmed(), &[server_title_change("server")]);
    let task = only_task(&state);

    assert_eq!(task.title, "server");
    assert_eq!(task.priority, Priority::Medium);
    assert_eq!(task.state, TaskState::Open);
}

#[test]
fn unacknowledged_local_edit_beats_server_change() {
    let state = visible_state(
        &confirmed(),
        &[server_title_change("server")],
        &[],
        &[update_task_operation()],
    );

    assert_eq!(only_task(&state).title, "local");
}

#[test]
fn acknowledged_local_edit_loses_to_server_change() {
    let acknowledged = [Uuid::parse_str(OPERATION_ID).unwrap()];
    let state = visible_state(
        &confirmed(),
        &[server_title_change("server")],
        &acknowledged,
        &[update_task_operation()],
    );

    assert_eq!(only_task(&state).title, "server");
}

#[test]
fn apply_changes_applies_section_position() {
    let state = apply_changes(&confirmed(), &[server_section_position_change("a1")]);

    assert_eq!(state.projects[0].sections[0].position.as_str(), "a1");
}

#[test]
fn acknowledged_section_move_keeps_server_position() {
    let acknowledged = [Uuid::parse_str(OPERATION_ID).unwrap()];
    let state = visible_state(
        &confirmed(),
        &[server_section_position_change("a1")],
        &acknowledged,
        &[move_section_operation("a1")],
    );

    assert_eq!(state.projects[0].sections[0].position.as_str(), "a1");
}
