use scry_core::position::{Position, PositionError, key_between, n_keys_between};

fn position(value: &str) -> Position {
    Position::parse(value).expect("test uses a valid position")
}

#[test]
fn key_between_leaves_both_bounds_open() {
    let key = key_between(None, None).unwrap();

    assert_eq!(key.as_str(), "a0");
}

#[test]
fn key_between_appends_after_a0() {
    let key = key_between(Some(&position("a0")), None).unwrap();

    assert_eq!(key.as_str(), "a1");
}

#[test]
fn key_between_places_a_key_between_a0_and_a1() {
    let key = key_between(Some(&position("a0")), Some(&position("a1"))).unwrap();

    assert_eq!(key.as_str(), "a0V");
}

#[test]
fn parse_rejects_an_invalid_key() {
    assert!(matches!(
        Position::parse("a00"),
        Err(PositionError::InvalidKey(_))
    ));
}

#[test]
fn key_between_rejects_equal_bounds() {
    let a0 = position("a0");

    assert_eq!(
        key_between(Some(&a0), Some(&a0)),
        Err(PositionError::InvalidBounds {
            before: "a0".to_string(),
            after: "a0".to_string(),
        })
    );
}

#[test]
fn n_keys_between_returns_three_ordered_keys() {
    let keys = n_keys_between(Some(&position("a0")), Some(&position("a1")), 3).unwrap();
    let keys: Vec<&str> = keys.iter().map(Position::as_str).collect();

    assert_eq!(keys, vec!["a0G", "a0V", "a0l"]);
}

#[test]
fn positions_sort_in_code_unit_order() {
    let mut keys: Vec<Position> = ["a1", "a0V", "Zz", "a0"]
        .iter()
        .map(|key| position(key))
        .collect();
    keys.sort();
    let keys: Vec<&str> = keys.iter().map(Position::as_str).collect();

    assert_eq!(keys, vec!["Zz", "a0", "a0V", "a1"]);
}
