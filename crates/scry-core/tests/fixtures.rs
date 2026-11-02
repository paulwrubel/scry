mod common;

use common::{
    FIXTURE_VERSION, REGISTERED_AREAS, fixture_areas_on_disk, fixture_dir, load_area,
    parse_fixture_file,
};
use std::collections::HashSet;

#[test]
fn every_fixture_directory_is_registered() {
    let discovered = fixture_areas_on_disk();
    let mut registered: Vec<String> = REGISTERED_AREAS
        .iter()
        .map(|area| area.to_string())
        .collect();
    registered.sort();
    assert_eq!(
        discovered, registered,
        "fixtures/ directories must match REGISTERED_AREAS in tests/common/mod.rs"
    );
}

#[test]
fn every_registered_area_loads() {
    for area in REGISTERED_AREAS {
        let path = fixture_dir().join(area).join("cases.json");
        assert!(path.is_file(), "missing fixture file {}", path.display());

        let file = load_area(area);
        assert_eq!(
            file.version,
            FIXTURE_VERSION,
            "wrong version in {}",
            path.display()
        );
        assert_eq!(
            file.area.as_str(),
            *area,
            "area mismatch in {}",
            path.display()
        );
        assert!(!file.cases.is_empty(), "{} has no cases", path.display());
    }
}

#[test]
fn case_names_are_unique_within_an_area() {
    for area in REGISTERED_AREAS {
        let file = load_area(area);
        let mut seen = HashSet::new();
        for case in &file.cases {
            assert!(
                seen.insert(case.name.as_str()),
                "duplicate case name `{}` in area `{area}`",
                case.name
            );
        }
    }
}

#[test]
fn case_input_and_expected_are_objects() {
    for area in REGISTERED_AREAS {
        let file = load_area(area);
        for case in &file.cases {
            assert!(
                case.input_object().is_some(),
                "case `{}` in area `{area}` has a non-object input",
                case.name
            );
            assert!(
                case.expected_object().is_some(),
                "case `{}` in area `{area}` has a non-object expected value",
                case.name
            );
        }
    }
}

#[test]
fn rejects_unknown_envelope_fields() {
    let text = r#"{
        "version": 1,
        "area": "positions",
        "cases": [],
        "unexpected": true
    }"#;

    let error = parse_fixture_file("positions", text).unwrap_err();
    assert!(error.contains("unknown field"), "unexpected error: {error}");
}

#[test]
fn rejects_unknown_case_fields() {
    let text = r#"{
        "version": 1,
        "area": "positions",
        "cases": [
            {
                "name": "case",
                "kind": "insert",
                "input": {},
                "expected": {},
                "unexpected": true
            }
        ]
    }"#;

    let error = parse_fixture_file("positions", text).unwrap_err();
    assert!(error.contains("unknown field"), "unexpected error: {error}");
}

#[test]
fn rejects_unknown_versions() {
    let text = r#"{
        "version": 2,
        "area": "positions",
        "cases": []
    }"#;

    let error = parse_fixture_file("positions", text).unwrap_err();
    assert!(error.contains("version 2"), "unexpected error: {error}");
    assert!(error.contains("version 1"), "unexpected error: {error}");
}

#[test]
fn rejects_a_mismatched_area() {
    let text = r#"{
        "version": 1,
        "area": "positions",
        "cases": []
    }"#;

    let error = parse_fixture_file("handles", text).unwrap_err();
    assert!(error.contains("`positions`"), "unexpected error: {error}");
    assert!(error.contains("`handles`"), "unexpected error: {error}");
}
