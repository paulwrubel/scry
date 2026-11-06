use scry_core::ids::{Handle, HandleError, shortest_handles};
use uuid::Uuid;

const SAMPLE_HEX: &str = "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c";
const SAMPLE_UUID: &str = "4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6c";

#[test]
fn parse_normalizes_compact_and_canonical_uuids() {
    for value in [
        "4F3A2B1C-9D2E-4F5A-8B6C-1D2E3F4A5B6C",
        "4F3A2B1C9D2E4F5A8B6C1D2E3F4A5B6C",
        SAMPLE_UUID,
        SAMPLE_HEX,
    ] {
        assert_eq!(value.parse::<Handle>().unwrap().as_str(), SAMPLE_HEX);
    }
}

#[test]
fn parse_rejects_too_short_input() {
    assert_eq!(
        "abc".parse::<Handle>(),
        Err(HandleError::TooShort { length: 3 })
    );
}

#[test]
fn parse_rejects_invalid_characters() {
    assert_eq!(
        "zzzz".parse::<Handle>(),
        Err(HandleError::InvalidCharacter {
            character: 'z',
            index: 0,
        })
    );
}

#[test]
fn parse_rejects_misplaced_dashes() {
    assert_eq!(
        "4f3a-2b1c".parse::<Handle>(),
        Err(HandleError::MisplacedDash { index: 4 })
    );
}

#[test]
fn shortest_handles_covers_a_small_fixed_set() {
    let ids = [
        Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap(),
        Uuid::parse_str(SAMPLE_UUID).unwrap(),
        Uuid::parse_str("4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6d").unwrap(),
        Uuid::parse_str("ffffffff-ffff-4fff-8fff-ffffffffffff").unwrap(),
    ];

    let handles = shortest_handles(ids.iter().copied(), 4);

    assert_eq!(handles[&ids[0]], "0000");
    assert_eq!(handles[&ids[1]], SAMPLE_HEX);
    assert_eq!(handles[&ids[2]], "4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6d");
    assert_eq!(handles[&ids[3]], "ffff");
}
