//! Writes the committed cross-language fixtures.
//!
//! Run with `just generate-fixtures`. The output is deterministic: cases are
//! listed explicitly, and generated sequences are recorded in order.

use scry_core::position::{Position, key_between};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

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
