mod common;

use common::{FixtureCase, load_area};
use scry_core::ids::{Handle, HandleError, shortest_handles};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizeInput {
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShortestInput {
    ids: Vec<String>,
    min_length: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveInput {
    ids: Vec<String>,
    handle: String,
}

#[test]
fn handles_match_the_shared_fixtures() {
    let file = load_area("handles");

    for case in &file.cases {
        match case.kind.as_str() {
            "normalize" => check_normalize(case),
            "shortest" => check_shortest(case),
            "resolve" => check_resolve(case),
            other => panic!("case `{}` uses unknown fixture kind `{other}`", case.name),
        }
    }
}

fn check_normalize(case: &FixtureCase) {
    let input: NormalizeInput = deserialize(case);
    match input.value.parse::<Handle>() {
        Ok(handle) => assert_eq!(
            handle.as_str(),
            expected_string(case, "hex"),
            "case `{}`",
            case.name
        ),
        Err(error) => assert_error(case, handle_error_code(&error)),
    }
}

fn check_shortest(case: &FixtureCase) {
    let input: ShortestInput = deserialize(case);
    let mut ids: Vec<Uuid> = input
        .ids
        .iter()
        .map(|id| Uuid::parse_str(id).expect("fixture uses valid uuids"))
        .collect();
    ids.sort();
    ids.dedup();

    let handles = shortest_handles(ids.iter().copied(), input.min_length);
    let mut actual: Vec<Value> = ids
        .iter()
        .map(|id| {
            json!({
                "id": id.to_string(),
                "handle": handles.get(id).expect("every id in the set has a handle"),
            })
        })
        .collect();
    actual.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    let expected = case
        .expected
        .get("handles")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("case `{}` is missing `handles`", case.name));

    assert_eq!(&actual, expected, "case `{}`", case.name);
}

fn check_resolve(case: &FixtureCase) {
    let input: ResolveInput = deserialize(case);
    let handle: Handle = match input.handle.parse() {
        Ok(handle) => handle,
        Err(error) => return assert_error(case, handle_error_code(&error)),
    };

    let mut matches: Vec<String> = input
        .ids
        .iter()
        .filter(|id| {
            Uuid::parse_str(id)
                .expect("fixture uses valid uuids")
                .simple()
                .to_string()
                .starts_with(handle.as_str())
        })
        .cloned()
        .collect();
    matches.sort();
    matches.dedup();

    let expected = case
        .expected
        .get("matches")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("case `{}` is missing `matches`", case.name));

    assert_eq!(
        json!(matches),
        Value::Array(expected.clone()),
        "case `{}`",
        case.name
    );
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

fn assert_error(case: &FixtureCase, actual: &str) {
    assert_eq!(
        actual,
        expected_string(case, "error"),
        "case `{}`",
        case.name
    );
}

fn expected_string<'a>(case: &'a FixtureCase, field: &str) -> &'a str {
    case.expected
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("case `{}` is missing a string `{field}`", case.name))
}

fn deserialize<T>(case: &FixtureCase) -> T
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(case.input.clone())
        .unwrap_or_else(|error| panic!("case `{}` has an invalid input: {error}", case.name))
}
