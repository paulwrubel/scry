mod common;

use common::{FixtureCase, load_area};
use scry_core::position::{Position, PositionError, key_between, n_keys_between};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsInput {
    before: Option<String>,
    after: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NKeysInput {
    before: Option<String>,
    after: Option<String>,
    n: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NarrowingInput {
    before: String,
    after: String,
    iterations: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderingInput {
    keys: Vec<String>,
}

#[test]
fn positions_match_the_shared_fixtures() {
    let file = load_area("positions");

    for case in &file.cases {
        match case.kind.as_str() {
            "key_between" => check_key_between(case),
            "n_keys_between" => check_n_keys_between(case),
            "narrowing" => check_narrowing(case),
            "ordering" => check_ordering(case),
            other => panic!("case `{}` uses unknown fixture kind `{other}`", case.name),
        }
    }
}

fn check_key_between(case: &FixtureCase) {
    let input: BoundsInput = deserialize(case);
    let before = match parse_bounds(&input.before, &input.after) {
        Ok(bounds) => bounds,
        Err(code) => return assert_error(case, code),
    };
    let (before, after) = before;

    match key_between(before.as_ref(), after.as_ref()) {
        Ok(key) => {
            let expected = expected_string(case, "key");
            assert_eq!(key.as_str(), expected, "case `{}`", case.name);
            assert_within(&input.before, &input.after, key.as_str());
        }
        Err(error) => assert_error(case, position_error_code(&error)),
    }
}

fn check_n_keys_between(case: &FixtureCase) {
    let input: NKeysInput = deserialize(case);
    let (before, after) = match parse_bounds(&input.before, &input.after) {
        Ok(bounds) => bounds,
        Err(code) => return assert_error(case, code),
    };

    match n_keys_between(before.as_ref(), after.as_ref(), input.n) {
        Ok(keys) => {
            let actual: Vec<String> = keys.iter().map(|key| key.as_str().to_string()).collect();
            assert_eq!(
                actual,
                expected_strings(case, "keys"),
                "case `{}`",
                case.name
            );
            assert_strictly_increasing(&actual);
            for key in &actual {
                assert_within(&input.before, &input.after, key);
            }
        }
        Err(error) => assert_error(case, position_error_code(&error)),
    }
}

fn check_narrowing(case: &FixtureCase) {
    let input: NarrowingInput = deserialize(case);
    let low = Position::parse(&input.before).expect("fixture uses a valid lower bound");
    let mut high = Position::parse(&input.after).expect("fixture uses a valid upper bound");
    let mut keys = Vec::with_capacity(input.iterations);

    for _ in 0..input.iterations {
        let next = key_between(Some(&low), Some(&high)).expect("the narrowing gap stays open");
        keys.push(next.as_str().to_string());
        high = next;
    }

    assert_eq!(keys, expected_strings(case, "keys"), "case `{}`", case.name);
}

fn check_ordering(case: &FixtureCase) {
    let input: OrderingInput = deserialize(case);
    let mut keys: Vec<Position> = input
        .keys
        .iter()
        .map(|key| Position::parse(key).expect("fixture uses valid ordering keys"))
        .collect();
    keys.sort();
    let actual: Vec<String> = keys.iter().map(|key| key.as_str().to_string()).collect();

    assert_eq!(
        actual,
        expected_strings(case, "sorted"),
        "case `{}`",
        case.name
    );
}

fn parse_bounds(
    before: &Option<String>,
    after: &Option<String>,
) -> Result<(Option<Position>, Option<Position>), &'static str> {
    Ok((parse_bound(before)?, parse_bound(after)?))
}

fn parse_bound(value: &Option<String>) -> Result<Option<Position>, &'static str> {
    value
        .as_deref()
        .map(Position::parse)
        .transpose()
        .map_err(|_| "invalid_key")
}

fn position_error_code(error: &PositionError) -> &'static str {
    match error {
        PositionError::InvalidKey(_) => "invalid_key",
        PositionError::InvalidBounds { .. } => "invalid_bounds",
        PositionError::CannotIncrement => "cannot_increment",
        PositionError::CannotDecrement => "cannot_decrement",
    }
}

fn assert_error(case: &FixtureCase, actual: &str) {
    let expected = expected_string(case, "error");
    assert_eq!(actual, expected, "case `{}`", case.name);
}

fn assert_within(before: &Option<String>, after: &Option<String>, key: &str) {
    let key = Position::parse(key).expect("generated keys are valid");
    let mut low = before.clone();
    let mut high = after.clone();
    if let (Some(low_ref), Some(high_ref)) = (&low, &high)
        && low_ref > high_ref
    {
        std::mem::swap(&mut low, &mut high);
    }

    if let Some(low) = low {
        let low = Position::parse(low).expect("fixture uses a valid lower bound");
        assert!(low < key, "generated key {key} is not above {low}");
    }
    if let Some(high) = high {
        let high = Position::parse(high).expect("fixture uses a valid upper bound");
        assert!(key < high, "generated key {key} is not below {high}");
    }
}

fn assert_strictly_increasing(keys: &[String]) {
    for pair in keys.windows(2) {
        let left = Position::parse(&pair[0]).expect("generated keys are valid");
        let right = Position::parse(&pair[1]).expect("generated keys are valid");
        assert!(left < right, "keys {left} and {right} are out of order");
    }
}

fn expected_string<'a>(case: &'a FixtureCase, field: &str) -> &'a str {
    case.expected
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("case `{}` is missing a string `{field}`", case.name))
}

fn expected_strings(case: &FixtureCase, field: &str) -> Vec<String> {
    case.expected
        .get(field)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("case `{}` is missing an array `{field}`", case.name))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("case `{}` has a non-string in `{field}`", case.name))
                .to_string()
        })
        .collect()
}

fn deserialize<T>(case: &FixtureCase) -> T
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(case.input.clone())
        .unwrap_or_else(|error| panic!("case `{}` has an invalid input: {error}", case.name))
}
