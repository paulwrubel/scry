#![allow(dead_code)]

use serde::Deserialize;
use std::path::PathBuf;

pub const FIXTURE_VERSION: u32 = 1;

// areas are registered here before their cases are exercised, so a directory
// added to fixtures/ without a matching entry fails the contract tests
pub const REGISTERED_AREAS: &[&str] = &["positions"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFile {
    pub version: u32,
    pub area: String,
    pub description: Option<String>,
    pub cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureCase {
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub input: serde_json::Value,
    pub expected: serde_json::Value,
}

impl FixtureCase {
    /// The case input as a JSON object, or `None` when the fixture is malformed.
    pub fn input_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.input.as_object()
    }

    /// The expected value as a JSON object, or `None` when the fixture is malformed.
    pub fn expected_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.expected.as_object()
    }
}

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Directory names under `fixtures/`, sorted. A missing directory yields no
/// areas rather than an error so a fresh checkout can run the contract tests.
pub fn fixture_areas_on_disk() -> Vec<String> {
    let dir = fixture_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => panic!(
            "failed to read fixture directory {}: {error}",
            dir.display()
        ),
    };

    let mut areas = Vec::new();
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| panic!("failed to read entry in {}: {error}", dir.display()));
        if entry.path().is_dir() {
            areas.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    areas.sort();
    areas
}

pub fn parse_fixture_file(area: &str, text: &str) -> Result<FixtureFile, String> {
    let file: FixtureFile = serde_json::from_str(text)
        .map_err(|error| format!("invalid fixture JSON for area `{area}`: {error}"))?;
    if file.version != FIXTURE_VERSION {
        return Err(format!(
            "unsupported fixture version {} for area `{area}`; this loader supports version {FIXTURE_VERSION}",
            file.version
        ));
    }
    if file.area != area {
        return Err(format!(
            "fixture area mismatch: file declares `{}` but loader expected `{area}`",
            file.area
        ));
    }
    Ok(file)
}

pub fn load_area(area: &str) -> FixtureFile {
    let path = fixture_dir().join(area).join("cases.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture file {}: {error}", path.display()));
    parse_fixture_file(area, &text)
        .unwrap_or_else(|error| panic!("failed to parse fixture file {}: {error}", path.display()))
}
