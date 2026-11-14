use chrono::{DateTime, Utc};
use scry_core::models::{AutomationAction, AutomationTrigger, Change, Priority, Task, TaskState};
use scry_core::sync::operations::{Operation, OperationKind};
use scry_core::sync::replay::{ClientState, apply_changes, replay_operations, visible_state};
use serde_json::json;
use uuid::Uuid;

const PROJECT_ID: &str = "11111111-1111-4111-8111-111111111111";
const SECTION_ID: &str = "22222222-2222-4222-8222-222222222222";
const TASK_ID: &str = "44444444-4444-4444-8444-444444444444";
const NEW_TASK_ID: &str = "55555555-5555-4555-8555-555555555555";
const COMMENT_ID: &str = "66666666-6666-4666-8666-666666666666";
const AUTOMATION_ID: &str = "88888888-8888-4888-8888-888888888888";
const OPERATION_ID: &str = "c0000000-0000-4000-8000-000000000001";
const NOW: &str = "2026-01-01T00:00:00Z";
const EDITED: &str = "2026-02-02T00:00:00Z";

fn utc(value: &str) -> DateTime<Utc> {
    value.parse().expect("valid timestamp")
}

fn confirmed() -> ClientState {
    serde_json::from_value(json!({
        "projects": [{
            "id": PROJECT_ID,
            "name": "alpha",
            "description": null,
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
                "priority": "normal",
                "section_id": SECTION_ID,
                "position": "a0",
                "state": "open",
                "tags": ["one"],
                "created_at": NOW
            }],
            "comments": [{
                "id": COMMENT_ID,
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
        "priority": "elevated",
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

fn create_automation_operation() -> Operation {
    serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "create_automation",
        "id": AUTOMATION_ID,
        "project_id": PROJECT_ID,
        "trigger": { "kind": "task_enters_section", "section_id": SECTION_ID },
        "actions": [{ "kind": "move_to_section", "section_id": SECTION_ID }],
        "created_at": NOW
    }))
    .unwrap()
}

fn update_automation_operation() -> Operation {
    serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "update_automation",
        "id": AUTOMATION_ID,
        "actions": [{ "kind": "set_priority", "priority": "elevated" }]
    }))
    .unwrap()
}

fn delete_automation_operation() -> Operation {
    serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "delete_automation",
        "id": AUTOMATION_ID
    }))
    .unwrap()
}

fn automation_change(kind: &str, field_values: serde_json::Value) -> Change {
    serde_json::from_value(json!({
        "seq": 2,
        "entity_type": "automation",
        "entity_id": AUTOMATION_ID,
        "kind": kind,
        "operation_id": OPERATION_ID,
        "client_timestamp": null,
        "field_values": field_values,
        "created_at": NOW
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
    assert_eq!(task.priority, Priority::Elevated);
    assert_eq!(task.state, TaskState::Closed);
    assert_eq!(state.projects[0].comments[0].contents, "note one");
    assert_eq!(only_task(&confirmed).title, "first");
}

#[test]
fn apply_changes_applies_update_task() {
    let state = apply_changes(&confirmed(), &[server_title_change("server")]);
    let task = only_task(&state);

    assert_eq!(task.title, "server");
    assert_eq!(task.priority, Priority::Normal);
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

#[test]
fn replay_operations_creates_automation() {
    let state = replay_operations(&confirmed(), &[create_automation_operation()]);
    let automation = &state.projects[0].automations[0];

    assert_eq!(automation.id, Uuid::parse_str(AUTOMATION_ID).unwrap());
    assert_eq!(automation.project_id, Uuid::parse_str(PROJECT_ID).unwrap());
    assert_eq!(
        automation.trigger,
        AutomationTrigger::TaskEntersSection {
            section_id: Uuid::parse_str(SECTION_ID).unwrap()
        }
    );
    assert_eq!(
        automation.actions,
        vec![AutomationAction::MoveToSection {
            section_id: Some(Uuid::parse_str(SECTION_ID).unwrap())
        }]
    );
}

#[test]
fn replay_operations_updates_automation_actions() {
    let confirmed = replay_operations(&confirmed(), &[create_automation_operation()]);
    let state = replay_operations(&confirmed, &[update_automation_operation()]);

    assert_eq!(
        state.projects[0].automations[0].actions,
        vec![AutomationAction::SetPriority {
            priority: Priority::Elevated
        }]
    );
}

#[test]
fn replay_operations_deletes_automation() {
    let confirmed = replay_operations(&confirmed(), &[create_automation_operation()]);
    let state = replay_operations(&confirmed, &[delete_automation_operation()]);

    assert!(state.projects[0].automations.is_empty());
}

#[test]
fn apply_changes_creates_updates_and_deletes_automation() {
    let created = apply_changes(
        &confirmed(),
        &[automation_change(
            "create",
            json!({
                "id": AUTOMATION_ID,
                "project_id": PROJECT_ID,
                "trigger": { "kind": "task_enters_section", "section_id": SECTION_ID },
                "actions": [{ "kind": "move_to_section", "section_id": SECTION_ID }],
                "created_at": NOW
            }),
        )],
    );
    assert_eq!(created.projects[0].automations.len(), 1);
    assert_eq!(
        created.projects[0].automations[0].actions,
        vec![AutomationAction::MoveToSection {
            section_id: Some(Uuid::parse_str(SECTION_ID).unwrap())
        }]
    );

    let updated = apply_changes(
        &created,
        &[automation_change(
            "update",
            json!({ "actions": [{ "kind": "open_task" }] }),
        )],
    );
    assert_eq!(
        updated.projects[0].automations[0].actions,
        vec![AutomationAction::OpenTask]
    );

    let deleted = apply_changes(&updated, &[automation_change("delete", json!({}))]);
    assert!(deleted.projects[0].automations.is_empty());
}

#[test]
fn old_create_project_operation_with_task_sorting_mode_deserializes() {
    // serde ignores fields the operation type no longer declares, so an
    // outbox entry queued before the field was removed still replays
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "create_project",
        "id": PROJECT_ID,
        "name": "alpha",
        "description": null,
        "task_sorting_mode": "priority",
        "show_priority": false,
        "created_at": NOW
    }))
    .expect("an old create_project carrying task_sorting_mode should deserialize");

    assert!(matches!(
        operation.kind,
        OperationKind::CreateProject { .. }
    ));
}

#[test]
fn old_update_project_operation_with_task_sorting_mode_deserializes() {
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "update_project",
        "id": PROJECT_ID,
        "task_sorting_mode": "priority"
    }))
    .expect("an old update_project carrying task_sorting_mode should deserialize");

    assert!(matches!(
        operation.kind,
        OperationKind::UpdateProject { .. }
    ));
}

#[test]
fn old_create_project_operation_with_show_priority_deserializes() {
    // serde ignores fields the operation type no longer declares, so an
    // outbox entry queued while projects still persisted show_priority replays
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "create_project",
        "id": PROJECT_ID,
        "name": "alpha",
        "description": null,
        "show_priority": false,
        "created_at": NOW
    }))
    .expect("an old create_project carrying show_priority should deserialize");

    assert!(matches!(
        operation.kind,
        OperationKind::CreateProject { .. }
    ));
}

#[test]
fn old_update_project_operation_with_show_priority_deserializes() {
    // same forward-compatibility guarantee for updates: queued ops still replay
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "update_project",
        "id": PROJECT_ID,
        "show_priority": true
    }))
    .expect("an old update_project carrying show_priority should deserialize");

    assert!(matches!(
        operation.kind,
        OperationKind::UpdateProject { .. }
    ));
}

#[test]
fn replay_operations_applies_updated_at_on_update_and_move() {
    let update: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "update_task",
        "id": TASK_ID,
        "title": "local",
        "updated_at": EDITED
    }))
    .unwrap();
    let state = replay_operations(&confirmed(), &[update]);
    assert_eq!(only_task(&state).updated_at, utc(EDITED));

    let moved: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "move_task",
        "id": TASK_ID,
        "section_id": null,
        "position": "a1",
        "updated_at": "2026-03-03T00:00:00Z"
    }))
    .unwrap();
    let state = replay_operations(&confirmed(), &[moved]);
    assert_eq!(only_task(&state).updated_at, utc("2026-03-03T00:00:00Z"));
}

#[test]
fn apply_changes_applies_task_updated_at() {
    let change: Change = serde_json::from_value(json!({
        "seq": 2,
        "entity_type": "task",
        "entity_id": TASK_ID,
        "kind": "update",
        "operation_id": OPERATION_ID,
        "client_timestamp": null,
        "field_values": { "updated_at": EDITED },
        "created_at": NOW
    }))
    .unwrap();

    let state = apply_changes(&confirmed(), &[change]);

    assert_eq!(only_task(&state).updated_at, utc(EDITED));
}

#[test]
fn create_task_with_updated_at_uses_the_operation_value() {
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "create_task",
        "id": NEW_TASK_ID,
        "project_id": PROJECT_ID,
        "title": "new",
        "description": null,
        "priority": "normal",
        "section_id": null,
        "position": "a1",
        "state": "open",
        "tags": [],
        "created_at": NOW,
        "updated_at": EDITED
    }))
    .unwrap();

    let state = replay_operations(&confirmed(), &[operation]);
    let task = state.projects[0]
        .tasks
        .iter()
        .find(|task| task.id == Uuid::parse_str(NEW_TASK_ID).unwrap())
        .expect("the task was created");

    assert_eq!(task.created_at, utc(NOW));
    assert_eq!(task.updated_at, utc(EDITED));
}

#[test]
fn create_task_without_updated_at_falls_back_to_created_at() {
    let operation: Operation = serde_json::from_value(json!({
        "operation_id": OPERATION_ID,
        "operation_version": 1,
        "kind": "create_task",
        "id": NEW_TASK_ID,
        "project_id": PROJECT_ID,
        "title": "new",
        "description": null,
        "priority": "normal",
        "section_id": null,
        "position": "a1",
        "state": "open",
        "tags": [],
        "created_at": NOW
    }))
    .unwrap();

    let state = replay_operations(&confirmed(), &[operation]);
    let task = state.projects[0]
        .tasks
        .iter()
        .find(|task| task.id == Uuid::parse_str(NEW_TASK_ID).unwrap())
        .expect("the task was created");

    assert_eq!(task.created_at, utc(NOW));
    assert_eq!(task.updated_at, utc(NOW));
}
