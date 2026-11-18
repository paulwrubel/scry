//! RRULE parsing, validation, canonical formatting, and the stored JSON shape.

use super::*;

/// Unsupported or malformed rules are rejected instead of guessed.
#[test]
fn parser_rejects_bad_rules() {
    for rrule in [
        "FREQ=DAILY;FOO=1",
        "COUNT=2",
        "FREQ=DAILY;COUNT=2;UNTIL=20260101",
        "FREQ=DAILY;INTERVAL=0",
        "FREQ=WEEKLY;BYDAY=1MO",
        "FREQ=DAILY;UNTIL=2026-01-01",
    ] {
        assert!(
            Recurrence::parse(rrule).is_err(),
            "{rrule} should be rejected"
        );
    }
}

/// The optional RRULE: prefix is accepted in either case.
#[test]
fn parser_accepts_rrule_prefix() {
    assert!(Recurrence::parse("RRULE:FREQ=DAILY").is_ok());
    assert!(Recurrence::parse("rrule:freq=daily").is_ok());
}

/// Formatting emits a canonical order and keeps ordinal BYDAY tokens.
#[test]
fn formats_a_rule_as_canonical_rrule_text() {
    let rule = Recurrence::parse(
        "RRULE:FREQ=MONTHLY;INTERVAL=2;BYMONTH=3,11;BYMONTHDAY=-1;BYDAY=1MO,-1FR;WKST=SU;COUNT=5",
    )
    .unwrap();

    assert_eq!(
        rule.to_rrule(),
        "FREQ=MONTHLY;INTERVAL=2;BYMONTH=3,11;BYMONTHDAY=-1;BYDAY=1MO,-1FR;WKST=SU;COUNT=5"
    );
    assert_eq!(format!("{rule}"), rule.to_rrule());
}

/// Default INTERVAL, WKST, and input casing are normalized away.
#[test]
fn formatting_drops_defaults_and_canonicalizes_input() {
    let rule = Recurrence::parse("rrule:freq=weekly;wkst=mo").unwrap();
    assert_eq!(rule.to_rrule(), "FREQ=WEEKLY");
}

/// Both UNTIL forms survive parse and format unchanged.
#[test]
fn formatting_round_trips_both_until_forms() {
    let date_until = Recurrence::parse("FREQ=DAILY;UNTIL=20261231").unwrap();
    assert_eq!(date_until.to_rrule(), "FREQ=DAILY;UNTIL=20261231");

    let instant_until = Recurrence::parse("FREQ=DAILY;UNTIL=20301231T235959Z").unwrap();
    assert_eq!(
        instant_until.to_rrule(),
        "FREQ=DAILY;UNTIL=20301231T235959Z"
    );

    for rule in [date_until, instant_until] {
        assert_eq!(Recurrence::parse(&rule.to_rrule()).unwrap(), rule);
    }
}

/// An EXDATE of another kind is rejected before any occurrence is resolved.
#[test]
fn validate_rejects_exdate_kind_mismatch() {
    let rec = RRule {
        text: "FREQ=DAILY".to_owned(),
        exdates: vec![When::Instant {
            timestamp: ts("2026-11-02T00:00:00Z"),
        }],
        rdates: Vec::new(),
    };
    let start = When::Date {
        date: date(2026, 11, 1),
    };

    assert!(matches!(
        rec.validate(&start),
        Err(RecurError::ExdateKindMismatch)
    ));
}

/// The stored value round-trips and omits an empty EXDATE list.
#[test]
fn rrule_json_round_trips() {
    let rec = RRule {
        text: "FREQ=WEEKLY;BYDAY=MO".to_owned(),
        exdates: vec![When::Date {
            date: date(2026, 11, 2),
        }],
        rdates: Vec::new(),
    };
    let value = serde_json::to_value(&rec).unwrap();
    assert_eq!(serde_json::from_value::<RRule>(value).unwrap(), rec);

    let empty = rrule("FREQ=DAILY");
    let value = serde_json::to_value(&empty).unwrap();
    assert!(value.get("exdates").is_none());
}
