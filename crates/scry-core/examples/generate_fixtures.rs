//! Writes the committed cross-language fixtures.
//!
//! Run with `just generate-fixtures`. The output is deterministic: cases are
//! listed explicitly, and generated sequences are recorded in order.

use scry_core::ids::{Handle, HandleError, MAX_HANDLE_LENGTH, MIN_HANDLE_LENGTH, shortest_handles};
use scry_core::models::Change;
use scry_core::position::{Position, key_between};
use scry_core::sync::operations::Operation;
use scry_core::sync::replay::{ClientState, apply_changes, replay_operations, visible_state};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SECOND_MAX: &str = "zzzzzzzzzzzzzzzzzzzzzzzzzzy";
const MAX: &str = "zzzzzzzzzzzzzzzzzzzzzzzzzzz";
const SMALLEST: &str = "A00000000000000000000000000";
const ABOVE_SMALLEST: &str = "A000000000000000000000000001";

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    write_area(
        &root,
        "positions",
        "Fractional-index keys: generation, bulk insertion, narrowing gaps, and ordering.",
        position_cases(),
    );
    write_area(
        &root,
        "handles",
        "Handle normalization, shortest distinguishing prefixes, and prefix resolution.",
        handle_cases(),
    );
    write_area(
        &root,
        "op-replay",
        "Client replay: confirmed state, pulled changes, unacknowledged outbox operations, and the resulting visible state.",
        op_replay_cases(),
    );
    write_area(
        &root,
        "task-order",
        "Task ordering: project sorting modes, deterministic ties, and closed tasks.",
        task_order_cases(),
    );
}

fn write_area(root: &Path, area: &str, description: &str, cases: Vec<Value>) {
    let dir = root.join(area);
    fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", dir.display()));
    let envelope = json!({
        "version": 1,
        "area": area,
        "description": description,
        "cases": cases,
    });
    let text = serde_json::to_string_pretty(&envelope).expect("fixtures must serialize");
    let path = dir.join("cases.json");
    fs::write(&path, format!("{text}\n"))
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
    println!("wrote {}", path.display());
}

fn position_cases() -> Vec<Value> {
    let mut cases = vec![
        key_case("both-open", None, None, "a0"),
        key_case("prepend-before-a0", None, Some("a0"), "Zz"),
        key_case("prepend-before-Zz", None, Some("Zz"), "Zy"),
        key_case("append-after-a0", Some("a0"), None, "a1"),
        key_case("append-after-a1", Some("a1"), None, "a2"),
        key_case("between-a0-a1", Some("a0"), Some("a1"), "a0V"),
        key_case("between-a1-a2", Some("a1"), Some("a2"), "a1V"),
        key_case("between-a0V-a1", Some("a0V"), Some("a1"), "a0l"),
        key_case("between-Zz-a0", Some("Zz"), Some("a0"), "ZzV"),
        key_case("between-Zz-a1", Some("Zz"), Some("a1"), "a0"),
        key_case("prepend-before-Y00", None, Some("Y00"), "Xzzz"),
        key_case("append-after-bzz", Some("bzz"), None, "c000"),
        key_case("between-a0-a0V", Some("a0"), Some("a0V"), "a0G"),
        key_case("between-a0-a0G", Some("a0"), Some("a0G"), "a08"),
        key_case("between-b125-b129", Some("b125"), Some("b129"), "b127"),
        key_case("between-a0-a1V", Some("a0"), Some("a1V"), "a1"),
        key_case("between-Zz-a01", Some("Zz"), Some("a01"), "a0"),
        key_case("prepend-before-a0V", None, Some("a0V"), "a0"),
        key_case("prepend-before-b999", None, Some("b999"), "b99"),
        key_case("reversed-bounds-a1-a0", Some("a1"), Some("a0"), "a0V"),
        key_case("append-after-second-max", Some(SECOND_MAX), None, MAX),
        key_case(
            "append-after-max",
            Some(MAX),
            None,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzV",
        ),
        key_case(
            "insert-above-smallest",
            None,
            Some(ABOVE_SMALLEST),
            "A000000000000000000000000000V",
        ),
        key_error_case("invalid-before", Some("a00"), None, "invalid_key"),
        key_error_case(
            "invalid-before-with-after",
            Some("a00"),
            Some("a1"),
            "invalid_key",
        ),
        key_error_case("invalid-head", Some("0"), Some("1"), "invalid_key"),
        key_error_case(
            "smallest-integer-is-reserved",
            None,
            Some(SMALLEST),
            "invalid_key",
        ),
        key_error_case("equal-bounds", Some("a0"), Some("a0"), "invalid_bounds"),
        n_keys_case("n-keys-empty", None, None, 2, &["a0", "a1"]),
        n_keys_case("n-keys-append", Some("a1"), None, 2, &["a2", "a3"]),
        n_keys_case("n-keys-prepend", None, Some("a0"), 2, &["Zy", "Zz"]),
        n_keys_case("n-keys-between", Some("a1"), Some("a2"), 2, &["a1G", "a1V"]),
        n_keys_case(
            "n-keys-five",
            None,
            None,
            5,
            &["a0", "a1", "a2", "a3", "a4"],
        ),
        n_keys_case(
            "n-keys-five-between",
            Some("a0"),
            Some("a1"),
            5,
            &["a08", "a0G", "a0V", "a0d", "a0l"],
        ),
        n_keys_case(
            "n-keys-five-append",
            Some("a0"),
            None,
            5,
            &["a1", "a2", "a3", "a4", "a5"],
        ),
        n_keys_case(
            "n-keys-five-prepend",
            None,
            Some("a0"),
            5,
            &["Zv", "Zw", "Zx", "Zy", "Zz"],
        ),
        n_keys_case("n-keys-zero", None, None, 0, &[]),
        n_keys_error_case(
            "n-keys-equal-bounds",
            Some("a0"),
            Some("a0"),
            3,
            "invalid_bounds",
        ),
        n_keys_error_case("n-keys-invalid-before", Some("a00"), None, 2, "invalid_key"),
        narrowing_case("narrowing-gap", 8),
        ordering_case("ordering-is-code-unit-order", &["a1", "a0V", "Zz", "a0"]),
    ];

    // keep the declared order stable even if the list above is edited later
    cases.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });
    cases
}

fn key_case(name: &str, before: Option<&str>, after: Option<&str>, expected: &str) -> Value {
    json!({
        "name": name,
        "kind": "key_between",
        "input": { "before": before, "after": after },
        "expected": { "key": expected },
    })
}

fn key_error_case(name: &str, before: Option<&str>, after: Option<&str>, error: &str) -> Value {
    json!({
        "name": name,
        "kind": "key_between",
        "input": { "before": before, "after": after },
        "expected": { "error": error },
    })
}

fn n_keys_case(
    name: &str,
    before: Option<&str>,
    after: Option<&str>,
    n: usize,
    expected: &[&str],
) -> Value {
    json!({
        "name": name,
        "kind": "n_keys_between",
        "input": { "before": before, "after": after, "n": n },
        "expected": { "keys": expected },
    })
}

fn n_keys_error_case(
    name: &str,
    before: Option<&str>,
    after: Option<&str>,
    n: usize,
    error: &str,
) -> Value {
    json!({
        "name": name,
        "kind": "n_keys_between",
        "input": { "before": before, "after": after, "n": n },
        "expected": { "error": error },
    })
}

fn narrowing_case(name: &str, iterations: usize) -> Value {
    let low = Position::parse("a0").expect("a0 is a valid position");
    let mut high = Position::parse("a1").expect("a1 is a valid position");
    let mut keys = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let next = key_between(Some(&low), Some(&high)).expect("the gap must stay open");
        keys.push(next.as_str().to_string());
        high = next;
    }

    json!({
        "name": name,
        "kind": "narrowing",
        "input": { "before": "a0", "after": "a1", "iterations": iterations },
        "expected": { "keys": keys },
    })
}

fn ordering_case(name: &str, keys: &[&str]) -> Value {
    let mut sorted: Vec<String> = keys.iter().map(|key| key.to_string()).collect();
    sorted.sort();

    json!({
        "name": name,
        "kind": "ordering",
        "input": { "keys": keys },
        "expected": { "sorted": sorted },
    })
}

const HANDLE_IDS: &[&str] = &[
    "00000000-0000-4000-8000-000000000001",
    "4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6c",
    "4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6d",
    "4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b7c",
    "ffffffff-ffff-4fff-8fff-ffffffffffff",
];
const MAX_HANDLE_HEX: &str = "0123456789abcdef0123456789abcdef";
const TOO_LONG_HANDLE_HEX: &str = "0123456789abcdef0123456789abcdef0";

fn handle_cases() -> Vec<Value> {
    let mut cases = vec![
        normalize_ok("normalize-compact", "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c"),
        normalize_ok("normalize-uppercase", "4F3A2B1C9D2E4F5A8B6C1D2E3F4A5B6C"),
        normalize_ok(
            "normalize-canonical-uuid",
            "4F3A2B1C-9D2E-4F5A-8B6C-1D2E3F4A5B6C",
        ),
        normalize_ok("normalize-whitespace", "  4f3a2b1c-9d2e  "),
        normalize_ok("normalize-partial-dash-8", "4f3a2b1c-9d2e"),
        normalize_ok("normalize-partial-dash-12", "4f3a2b1c9d2e-4f5a"),
        normalize_ok("normalize-partial-dash-16", "4f3a2b1c9d2e4f5a-8b6c"),
        normalize_ok("normalize-partial-dash-20", "4f3a2b1c9d2e4f5a8b6c-1d2e"),
        normalize_ok("normalize-max-length", MAX_HANDLE_HEX),
        normalize_err("normalize-empty", ""),
        normalize_err("normalize-too-short", "abc"),
        normalize_err("normalize-too-long", TOO_LONG_HANDLE_HEX),
        normalize_err("normalize-invalid-character", "zzzz"),
        normalize_err("normalize-misplaced-dash", "4f3a-2b1c"),
        normalize_err("normalize-leading-dash", "-4f3a2b1c"),
        normalize_err("normalize-trailing-dash", "4f3a2b1c-"),
        shortest_case("shortest-min-four", HANDLE_IDS, MIN_HANDLE_LENGTH),
        shortest_case("shortest-min-eight", HANDLE_IDS, 8),
        shortest_case("shortest-min-zero-clamps", HANDLE_IDS, 0),
        shortest_case(
            "shortest-min-over-max-clamps",
            HANDLE_IDS,
            MAX_HANDLE_LENGTH + 8,
        ),
        shortest_case("shortest-single-id", &[HANDLE_IDS[1]], MIN_HANDLE_LENGTH),
        shortest_case(
            "shortest-duplicate-ids",
            &[HANDLE_IDS[1], HANDLE_IDS[1], HANDLE_IDS[2]],
            MIN_HANDLE_LENGTH,
        ),
        shortest_case(
            "shortest-shuffled-input",
            &[
                HANDLE_IDS[4],
                HANDLE_IDS[1],
                HANDLE_IDS[0],
                HANDLE_IDS[3],
                HANDLE_IDS[2],
            ],
            MIN_HANDLE_LENGTH,
        ),
        shortest_case("shortest-no-ids", &[], MIN_HANDLE_LENGTH),
        resolve_ok("resolve-ambiguous", HANDLE_IDS, "4f3a"),
        resolve_ok("resolve-uppercase-dashed", HANDLE_IDS, "4F3A2B1C-9D2E"),
        resolve_ok(
            "resolve-shared-prefix",
            HANDLE_IDS,
            "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6",
        ),
        resolve_ok("resolve-full-id", HANDLE_IDS, HANDLE_IDS[1]),
        resolve_ok("resolve-first-id", HANDLE_IDS, "0000"),
        resolve_ok("resolve-last-id", HANDLE_IDS, "ffff"),
        resolve_ok("resolve-no-match", HANDLE_IDS, "abcd"),
        resolve_err("resolve-invalid-character", HANDLE_IDS, "zz"),
        resolve_err("resolve-misplaced-dash", HANDLE_IDS, "4f3a-2b1c"),
    ];
    cases.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });
    cases
}

fn normalize_ok(name: &str, value: &str) -> Value {
    let handle: Handle = value
        .parse()
        .expect("normalize fixture uses a valid handle");
    json!({
        "name": name,
        "kind": "normalize",
        "input": { "value": value },
        "expected": { "hex": handle.as_str() },
    })
}

fn normalize_err(name: &str, value: &str) -> Value {
    let error = value
        .parse::<Handle>()
        .expect_err("normalize fixture uses an invalid handle");
    json!({
        "name": name,
        "kind": "normalize",
        "input": { "value": value },
        "expected": { "error": handle_error_code(&error) },
    })
}

fn shortest_case(name: &str, ids: &[&str], min_length: usize) -> Value {
    let mut unique: Vec<Uuid> = ids
        .iter()
        .map(|id| Uuid::parse_str(id).expect("shortest fixture uses a valid uuid"))
        .collect();
    unique.sort();
    unique.dedup();

    let handles = shortest_handles(unique.iter().copied(), min_length);
    let mut expected: Vec<Value> = unique
        .iter()
        .map(|id| {
            let handle = handles.get(id).expect("every id in the set has a handle");
            json!({ "id": id.to_string(), "handle": handle })
        })
        .collect();
    expected.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    json!({
        "name": name,
        "kind": "shortest",
        "input": { "ids": ids, "min_length": min_length },
        "expected": { "handles": expected },
    })
}

fn resolve_ok(name: &str, ids: &[&str], raw: &str) -> Value {
    let handle: Handle = raw.parse().expect("resolve fixture uses a valid handle");
    let mut matches: Vec<String> = ids
        .iter()
        .filter(|id| {
            Uuid::parse_str(id)
                .expect("resolve fixture uses a valid uuid")
                .simple()
                .to_string()
                .starts_with(handle.as_str())
        })
        .map(|id| id.to_string())
        .collect();
    matches.sort();
    matches.dedup();

    json!({
        "name": name,
        "kind": "resolve",
        "input": { "ids": ids, "handle": raw },
        "expected": { "matches": matches },
    })
}

fn resolve_err(name: &str, ids: &[&str], raw: &str) -> Value {
    let error = raw
        .parse::<Handle>()
        .expect_err("resolve fixture uses an invalid handle");
    json!({
        "name": name,
        "kind": "resolve",
        "input": { "ids": ids, "handle": raw },
        "expected": { "error": handle_error_code(&error) },
    })
}

fn handle_error_code(error: &HandleError) -> &'static str {
    match error {
        HandleError::Empty => "empty",
        HandleError::TooShort { .. } => "too_short",
        HandleError::TooLong { .. } => "too_long",
        HandleError::MisplacedDash { .. } => "misplaced_dash",
        HandleError::InvalidCharacter { .. } => "invalid_character",
    }
}

const PROJECT_ID: &str = "11111111-1111-4111-8111-111111111111";
const PROJECT_2: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const SECTION_A: &str = "22222222-2222-4222-8222-222222222222";
const SECTION_B: &str = "33333333-3333-4333-8333-333333333333";
const SECTION_C: &str = "99999999-9999-4999-8999-999999999999";
const TASK_1: &str = "44444444-4444-4444-8444-444444444444";
const TASK_2: &str = "55555555-5555-4555-8555-555555555555";
const TASK_3: &str = "88888888-8888-4888-8888-888888888888";
const ORDER_TASK_1: &str = "e0000000-0000-4000-8000-000000000001";
const ORDER_TASK_2: &str = "e0000000-0000-4000-8000-000000000002";
const ORDER_TASK_3: &str = "e0000000-0000-4000-8000-000000000003";
const ORDER_TASK_4: &str = "e0000000-0000-4000-8000-000000000004";
const ORDER_TASK_5: &str = "e0000000-0000-4000-8000-000000000005";
const NOTE_1: &str = "66666666-6666-4666-8666-666666666666";
const NOTE_2: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const CREATED_AT: &str = "2026-01-01T00:00:00Z";

const OP_LOCAL_TITLE: &str = "c0000000-0000-4000-8000-000000000001";
const OP_LOCAL_PRIORITY: &str = "c0000000-0000-4000-8000-000000000002";
const OP_LOCAL_OTHER: &str = "c0000000-0000-4000-8000-000000000003";
const OP_SERVER_ONE: &str = "d0000000-0000-4000-8000-000000000001";
const OP_SERVER_TWO: &str = "d0000000-0000-4000-8000-000000000002";

fn op_replay_cases() -> Vec<Value> {
    let mut cases = vec![
        replay_case(
            "replay-update-task-fields",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_TITLE,
                "operation_version": 1,
                "kind": "update_task",
                "id": TASK_1,
                "title": "local first",
                "priority": "critical",
                "tags": ["local"],
            }]),
        ),
        replay_case(
            "replay-move-task",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "move_task",
                "id": TASK_1,
                "section_id": SECTION_B,
                "position": "a2",
            }]),
        ),
        replay_case(
            "replay-create-task",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "create_task",
                "id": TASK_3,
                "project_id": PROJECT_ID,
                "title": "third",
                "description": "third description",
                "priority": "low",
                "section_id": SECTION_A,
                "position": "a1",
                "state": "open",
                "tags": ["new"],
                "created_at": CREATED_AT,
            }]),
        ),
        replay_case(
            "replay-delete-task-removes-notes",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "delete_task",
                "id": TASK_1,
            }]),
        ),
        replay_case(
            "replay-update-section-name-and-color",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "update_section",
                "id": SECTION_A,
                "name": "renamed",
                "color": { "set": "yellow" },
            }]),
        ),
        replay_case(
            "replay-clear-section-color",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "update_section",
                "id": SECTION_B,
                "color": "clear",
            }]),
        ),
        replay_case(
            "replay-delete-section-relocates-tasks",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "delete_section",
                "id": SECTION_A,
            }]),
        ),
        replay_case(
            "replay-create-section",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "create_section",
                "id": SECTION_C,
                "project_id": PROJECT_ID,
                "name": "later",
                "position": "a2",
                "color": "cyan",
                "created_at": CREATED_AT,
            }]),
        ),
        replay_case(
            "replay-update-note",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "update_note",
                "id": NOTE_1,
                "contents": "local note",
            }]),
        ),
        replay_case(
            "replay-create-note",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "create_note",
                "id": NOTE_2,
                "task_id": TASK_2,
                "contents": "second note",
                "created_at": CREATED_AT,
            }]),
        ),
        replay_case(
            "replay-delete-note",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "delete_note",
                "id": NOTE_1,
            }]),
        ),
        replay_case(
            "replay-update-project",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "update_project",
                "id": PROJECT_ID,
                "name": "alpha local",
                "description": "local description",
                "task_sorting_mode": "priority",
                "show_priority": false,
            }]),
        ),
        replay_case(
            "replay-create-project",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "create_project",
                "id": PROJECT_2,
                "name": "beta",
                "description": null,
                "task_sorting_mode": "manual",
                "show_priority": false,
                "created_at": CREATED_AT,
            }]),
        ),
        replay_case(
            "replay-delete-project",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_OTHER,
                "operation_version": 1,
                "kind": "delete_project",
                "id": PROJECT_ID,
            }]),
        ),
        replay_case(
            "replay-create-then-update-task",
            base_state(),
            json!([
                {
                    "operation_id": OP_LOCAL_OTHER,
                    "operation_version": 1,
                    "kind": "create_task",
                    "id": TASK_3,
                    "project_id": PROJECT_ID,
                    "title": "third",
                    "description": null,
                    "priority": "low",
                    "section_id": SECTION_A,
                    "position": "a1",
                    "state": "open",
                    "tags": [],
                    "created_at": CREATED_AT,
                },
                {
                    "operation_id": OP_LOCAL_TITLE,
                    "operation_version": 1,
                    "kind": "update_task",
                    "id": TASK_3,
                    "title": "third local",
                },
            ]),
        ),
        apply_changes_case(
            "apply-changes-update-task",
            base_state(),
            json!([change(
                2,
                "task",
                TASK_1,
                "update",
                OP_SERVER_ONE,
                json!({ "title": "server first", "priority": "low", "description": null }),
            )]),
        ),
        apply_changes_case(
            "apply-changes-create-task",
            base_state(),
            json!([change(
                2,
                "task",
                TASK_3,
                "create",
                OP_SERVER_ONE,
                task_3_value(),
            )]),
        ),
        apply_changes_case(
            "apply-changes-delete-task",
            base_state(),
            json!([change(
                2,
                "task",
                TASK_1,
                "delete",
                OP_SERVER_ONE,
                json!({}),
            )]),
        ),
        apply_changes_case(
            "apply-changes-update-project",
            base_state(),
            json!([change(
                2,
                "project",
                PROJECT_ID,
                "update",
                OP_SERVER_ONE,
                json!({ "name": "server alpha", "show_priority": false }),
            )]),
        ),
        apply_changes_case(
            "apply-changes-update-note",
            base_state(),
            json!([change(
                2,
                "note",
                NOTE_1,
                "update",
                OP_SERVER_ONE,
                json!({ "contents": "server note" }),
            )]),
        ),
        cycle_case(
            "cycle-lost-conflict-corrects-after-ack",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_TITLE,
                "operation_version": 1,
                "kind": "update_task",
                "id": TASK_1,
                "title": "local",
            }]),
            json!([change(
                2,
                "task",
                TASK_1,
                "update",
                OP_SERVER_ONE,
                json!({ "title": "server" }),
            )]),
            &[OP_LOCAL_TITLE],
        ),
        cycle_case(
            "cycle-unacked-local-edit-stays-visible",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_TITLE,
                "operation_version": 1,
                "kind": "update_task",
                "id": TASK_1,
                "title": "local",
            }]),
            json!([change(
                2,
                "task",
                TASK_1,
                "update",
                OP_SERVER_ONE,
                json!({ "title": "server" }),
            )]),
            &[],
        ),
        cycle_case(
            "cycle-delete-wins-over-unacked-edit",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_TITLE,
                "operation_version": 1,
                "kind": "update_task",
                "id": TASK_1,
                "title": "local",
            }]),
            json!([change(
                2,
                "task",
                TASK_1,
                "delete",
                OP_SERVER_ONE,
                json!({}),
            )]),
            &[OP_LOCAL_TITLE],
        ),
        cycle_case(
            "cycle-ack-without-conflict",
            base_state(),
            json!([{
                "operation_id": OP_LOCAL_TITLE,
                "operation_version": 1,
                "kind": "update_task",
                "id": TASK_1,
                "title": "local",
            }]),
            json!([change(
                2,
                "task",
                TASK_1,
                "update",
                OP_LOCAL_TITLE,
                json!({ "title": "local" }),
            )]),
            &[OP_LOCAL_TITLE],
        ),
        cycle_case(
            "cycle-remaining-outbox-replays-after-pull",
            base_state(),
            json!([
                {
                    "operation_id": OP_LOCAL_TITLE,
                    "operation_version": 1,
                    "kind": "update_task",
                    "id": TASK_1,
                    "title": "local title",
                },
                {
                    "operation_id": OP_LOCAL_PRIORITY,
                    "operation_version": 1,
                    "kind": "update_task",
                    "id": TASK_1,
                    "priority": "critical",
                },
            ]),
            json!([change(
                2,
                "task",
                TASK_1,
                "update",
                OP_SERVER_TWO,
                json!({ "title": "server title" }),
            )]),
            &[OP_LOCAL_TITLE],
        ),
        cycle_case(
            "cycle-create-note-then-update",
            base_state(),
            json!([
                {
                    "operation_id": OP_LOCAL_OTHER,
                    "operation_version": 1,
                    "kind": "create_note",
                    "id": NOTE_2,
                    "task_id": TASK_2,
                    "contents": "second note",
                    "created_at": CREATED_AT,
                },
                {
                    "operation_id": OP_LOCAL_TITLE,
                    "operation_version": 1,
                    "kind": "update_note",
                    "id": NOTE_2,
                    "contents": "local final",
                },
            ]),
            json!([]),
            &[],
        ),
        cycle_case(
            "cycle-delete-section-with-remaining-task-edit",
            base_state(),
            json!([
                {
                    "operation_id": OP_LOCAL_OTHER,
                    "operation_version": 1,
                    "kind": "delete_section",
                    "id": SECTION_A,
                },
                {
                    "operation_id": OP_LOCAL_TITLE,
                    "operation_version": 1,
                    "kind": "update_task",
                    "id": TASK_1,
                    "title": "local",
                },
            ]),
            json!([]),
            &[],
        ),
    ];
    cases.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });
    cases
}

fn replay_case(name: &str, confirmed: Value, ops: Value) -> Value {
    let state: ClientState =
        serde_json::from_value(confirmed.clone()).expect("replay case has a valid confirmed state");
    let operations: Vec<Operation> =
        serde_json::from_value(ops.clone()).expect("replay case has valid operations");
    let visible = replay_operations(&state, &operations);

    json!({
        "name": name,
        "kind": "replay",
        "input": { "confirmed": confirmed, "ops": ops },
        "expected": { "visible": serde_json::to_value(visible).expect("state serializes") },
    })
}

fn apply_changes_case(name: &str, confirmed: Value, changes: Value) -> Value {
    let state: ClientState =
        serde_json::from_value(confirmed.clone()).expect("change case has a valid confirmed state");
    let changes: Vec<Change> =
        serde_json::from_value(changes.clone()).expect("change case has valid changes");
    let confirmed_after = apply_changes(&state, &changes);

    json!({
        "name": name,
        "kind": "apply_changes",
        "input": { "confirmed": confirmed, "changes": changes },
        "expected": { "confirmed": serde_json::to_value(confirmed_after).expect("state serializes") },
    })
}

fn cycle_case(
    name: &str,
    confirmed: Value,
    ops: Value,
    changes: Value,
    acknowledged: &[&str],
) -> Value {
    let state: ClientState =
        serde_json::from_value(confirmed.clone()).expect("cycle case has a valid confirmed state");
    let operations: Vec<Operation> =
        serde_json::from_value(ops.clone()).expect("cycle case has valid operations");
    let changes: Vec<Change> =
        serde_json::from_value(changes.clone()).expect("cycle case has valid changes");
    let acked: Vec<Uuid> = acknowledged
        .iter()
        .map(|id| Uuid::parse_str(id).expect("cycle case has valid acknowledged ids"))
        .collect();
    let visible = visible_state(&state, &changes, &acked, &operations);

    json!({
        "name": name,
        "kind": "cycle",
        "input": {
            "confirmed": confirmed,
            "ops": ops,
            "changes": changes,
            "acked": acknowledged,
        },
        "expected": { "visible": serde_json::to_value(visible).expect("state serializes") },
    })
}

fn change(
    seq: i64,
    entity_type: &str,
    entity_id: &str,
    kind: &str,
    operation_id: &str,
    field_values: Value,
) -> Value {
    json!({
        "seq": seq,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "kind": kind,
        "operation_id": operation_id,
        "client_timestamp": null,
        "field_values": field_values,
        "created_at": CREATED_AT,
    })
}

fn base_state() -> Value {
    json!({
        "projects": [{
            "id": PROJECT_ID,
            "name": "alpha",
            "description": null,
            "task_sorting_mode": "manual",
            "show_priority": true,
            "created_at": CREATED_AT,
            "sections": [section_a_value(), section_b_value()],
            "tasks": [task_1_value(), task_2_value()],
            "notes": [note_1_value()],
        }]
    })
}

fn section_a_value() -> Value {
    json!({
        "id": SECTION_A,
        "project_id": PROJECT_ID,
        "name": "backlog",
        "position": "a0",
        "color": null,
        "created_at": CREATED_AT,
    })
}

fn section_b_value() -> Value {
    json!({
        "id": SECTION_B,
        "project_id": PROJECT_ID,
        "name": "doing",
        "position": "a1",
        "color": "blue",
        "created_at": CREATED_AT,
    })
}

fn task_1_value() -> Value {
    json!({
        "id": TASK_1,
        "project_id": PROJECT_ID,
        "title": "first",
        "description": "first description",
        "priority": "medium",
        "section_id": SECTION_A,
        "position": "a0",
        "state": "open",
        "tags": ["one"],
        "created_at": CREATED_AT,
    })
}

fn task_2_value() -> Value {
    json!({
        "id": TASK_2,
        "project_id": PROJECT_ID,
        "title": "second",
        "description": null,
        "priority": "high",
        "section_id": SECTION_B,
        "position": "a1",
        "state": "open",
        "tags": [],
        "created_at": CREATED_AT,
    })
}

fn task_3_value() -> Value {
    json!({
        "id": TASK_3,
        "project_id": PROJECT_ID,
        "title": "third",
        "description": "third description",
        "priority": "low",
        "section_id": SECTION_A,
        "position": "a1",
        "state": "open",
        "tags": ["new"],
        "created_at": CREATED_AT,
    })
}

fn note_1_value() -> Value {
    json!({
        "id": NOTE_1,
        "task_id": TASK_1,
        "contents": "note one",
        "created_at": CREATED_AT,
    })
}

fn task_order_cases() -> Vec<Value> {
    let tasks = || {
        vec![
            order_task(
                ORDER_TASK_1,
                "alpha",
                "minimal",
                "a1",
                "2026-01-01T00:00:00Z",
                "open",
            ),
            order_task(
                ORDER_TASK_2,
                "Beta",
                "high",
                "a0",
                "2026-01-03T00:00:00Z",
                "open",
            ),
            order_task(
                ORDER_TASK_3,
                "ALPHA",
                "critical",
                "a2",
                "2026-01-02T00:00:00Z",
                "open",
            ),
            order_task(
                ORDER_TASK_4,
                "beta",
                "low",
                "a0",
                "2026-01-02T00:00:00Z",
                "open",
            ),
            order_task(
                ORDER_TASK_5,
                "Beta",
                "high",
                "a0",
                "2026-01-01T00:00:00Z",
                "open",
            ),
        ]
    };

    let closed = tasks()
        .into_iter()
        .map(|task| with_state(task, "closed"))
        .collect();

    let mut cases = vec![
        sort_case(
            "manual-orders-by-position-then-id",
            "manual",
            tasks(),
            &[
                ORDER_TASK_2,
                ORDER_TASK_4,
                ORDER_TASK_5,
                ORDER_TASK_1,
                ORDER_TASK_3,
            ],
        ),
        sort_case(
            "alphabetical-orders-by-title-then-id",
            "alphabetical",
            tasks(),
            &[
                ORDER_TASK_3,
                ORDER_TASK_2,
                ORDER_TASK_5,
                ORDER_TASK_1,
                ORDER_TASK_4,
            ],
        ),
        sort_case(
            "alphabetical-case-insensitive-orders-by-lowercased-title-then-id",
            "alphabetical-case-insensitive",
            tasks(),
            &[
                ORDER_TASK_1,
                ORDER_TASK_3,
                ORDER_TASK_2,
                ORDER_TASK_4,
                ORDER_TASK_5,
            ],
        ),
        sort_case(
            "created-orders-by-created-at-then-id",
            "created",
            tasks(),
            &[
                ORDER_TASK_1,
                ORDER_TASK_5,
                ORDER_TASK_3,
                ORDER_TASK_4,
                ORDER_TASK_2,
            ],
        ),
        sort_case(
            "priority-orders-by-priority-then-id",
            "priority",
            tasks(),
            &[
                ORDER_TASK_3,
                ORDER_TASK_2,
                ORDER_TASK_5,
                ORDER_TASK_4,
                ORDER_TASK_1,
            ],
        ),
        sort_case(
            "closed-tasks-use-the-same-order",
            "priority",
            closed,
            &[
                ORDER_TASK_3,
                ORDER_TASK_2,
                ORDER_TASK_5,
                ORDER_TASK_4,
                ORDER_TASK_1,
            ],
        ),
    ];

    cases.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });
    cases
}

fn order_task(
    id: &str,
    title: &str,
    priority: &str,
    position: &str,
    created_at: &str,
    state: &str,
) -> Value {
    json!({
        "id": id,
        "project_id": PROJECT_ID,
        "title": title,
        "description": null,
        "priority": priority,
        "section_id": null,
        "position": position,
        "state": state,
        "tags": [],
        "created_at": created_at,
    })
}

fn with_state(mut task: Value, state: &str) -> Value {
    task["state"] = json!(state);
    task
}

fn sort_case(name: &str, mode: &str, tasks: Vec<Value>, expected: &[&str]) -> Value {
    json!({
        "name": name,
        "kind": "sort",
        "input": { "mode": mode, "tasks": tasks },
        "expected": { "ids": expected },
    })
}
