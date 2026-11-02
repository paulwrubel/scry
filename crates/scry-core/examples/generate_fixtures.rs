//! Writes the committed cross-language fixtures.
//!
//! Run with `just generate-fixtures`. The output is deterministic: cases are
//! listed explicitly, and generated sequences are recorded in order.

use scry_core::ids::{Handle, HandleError, MAX_HANDLE_LENGTH, MIN_HANDLE_LENGTH, shortest_handles};
use scry_core::position::{Position, key_between};
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
