//! Expansion behavior: DST across all four kinds, BY-rule selection, bounds,
//! and EXDATE removal.

use super::*;
use crate::recurrence::expand::CivilWhen;
use crate::temporal::Zoned;

/// A weekly zoned series keeps its 09:00 wall clock across spring-forward.
#[test]
fn weekly_zoned_keeps_wall_clock_across_dst() {
    let tz = zone("America/Chicago");
    let start = date(2026, 3, 1).at(9, 0, 0, 0);
    let end = date(2026, 3, 1).at(10, 0, 0, 0);
    let interval = Interval::new(
        When::Zoned(Zoned::new(start, "America/Chicago").unwrap()),
        When::Zoned(Zoned::new(end, "America/Chicago").unwrap()),
    )
    .unwrap();

    let got = occurrences(
        &interval,
        &rrule("FREQ=WEEKLY"),
        ts("2026-03-01T00:00:00Z"),
        ts("2026-03-10T00:00:00Z"),
        &tz,
    )
    .unwrap();

    assert_eq!(got.len(), 2);
    for (occurrence, expected) in got.iter().zip([date(2026, 3, 1), date(2026, 3, 8)]) {
        match occurrence.start() {
            When::Zoned(zoned) => assert_eq!(zoned.datetime(), expected.at(9, 0, 0, 0)),
            other => panic!("unexpected start {other:?}"),
        }
    }
    assert_eq!(
        got[0].start_instant(&tz).unwrap(),
        ts("2026-03-01T15:00:00Z")
    );
    assert_eq!(
        got[1].start_instant(&tz).unwrap(),
        ts("2026-03-08T14:00:00Z")
    );
}

/// A weekly date series stays on local midnight, which moves in UTC.
#[test]
fn weekly_date_midnight_shifts_with_dst() {
    let tz = zone("America/Chicago");
    let interval = all_day(date(2026, 3, 1), date(2026, 3, 2));

    let got = occurrences(
        &interval,
        &rrule("FREQ=WEEKLY"),
        ts("2026-02-01T00:00:00Z"),
        ts("2026-03-20T00:00:00Z"),
        &tz,
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 3, 1), date(2026, 3, 8), date(2026, 3, 15)]
    );
    assert_eq!(
        got[0].start_instant(&tz).unwrap(),
        ts("2026-03-01T06:00:00Z")
    );
    assert_eq!(
        got[2].start_instant(&tz).unwrap(),
        ts("2026-03-15T05:00:00Z")
    );
}

/// A floating series keeps its wall clock and follows the caller's DST rule.
#[test]
fn weekly_floating_keeps_wall_clock_and_shifts_instant() {
    let tz = zone("America/Chicago");
    let interval = Interval::new(
        When::Floating {
            datetime: date(2026, 3, 1).at(9, 0, 0, 0),
        },
        When::Floating {
            datetime: date(2026, 3, 1).at(10, 0, 0, 0),
        },
    )
    .unwrap();

    let got = occurrences(
        &interval,
        &rrule("FREQ=WEEKLY"),
        ts("2026-03-01T00:00:00Z"),
        ts("2026-03-10T00:00:00Z"),
        &tz,
    )
    .unwrap();

    assert_eq!(got.len(), 2);
    for occurrence in &got {
        match occurrence.start() {
            When::Floating { datetime } => {
                assert_eq!(datetime.time(), jiff::civil::time(9, 0, 0, 0));
            }
            other => panic!("unexpected start {other:?}"),
        }
    }
    assert_eq!(
        got[0].start_instant(&tz).unwrap(),
        ts("2026-03-01T15:00:00Z")
    );
    assert_eq!(
        got[1].start_instant(&tz).unwrap(),
        ts("2026-03-08T14:00:00Z")
    );
}

/// An instant series repeats at the same absolute time every week.
#[test]
fn weekly_instant_stays_in_utc() {
    let tz = zone("America/Chicago");
    let interval = Interval::new(
        When::Instant {
            timestamp: ts("2026-03-01T15:00:00Z"),
        },
        When::Instant {
            timestamp: ts("2026-03-01T16:00:00Z"),
        },
    )
    .unwrap();

    let got = occurrences(
        &interval,
        &rrule("FREQ=WEEKLY"),
        ts("2026-03-01T00:00:00Z"),
        ts("2026-03-10T00:00:00Z"),
        &tz,
    )
    .unwrap();

    assert_eq!(
        got.iter()
            .map(|occurrence| occurrence.start_instant(&tz).unwrap())
            .collect::<Vec<_>>(),
        vec![ts("2026-03-01T15:00:00Z"), ts("2026-03-08T15:00:00Z"),]
    );
}

/// An EXDATE removes exactly the occurrence starting at that value.
#[test]
fn exdate_removes_exactly_one_occurrence() {
    let rec = RRule {
        text: "FREQ=DAILY;COUNT=3".to_owned(),
        exdates: vec![When::Date {
            date: date(2026, 11, 2),
        }],
        rdates: Vec::new(),
    };
    let interval = all_day(date(2026, 11, 1), date(2026, 11, 2));

    let got = occurrences(
        &interval,
        &rec,
        ts("2026-11-01T00:00:00Z"),
        ts("2026-11-10T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 11, 1), date(2026, 11, 3)]
    );
}

/// COUNT stops after that many occurrences, including the base.
#[test]
fn count_stops_generation() {
    let interval = all_day(date(2026, 11, 1), date(2026, 11, 2));
    let got = occurrences(
        &interval,
        &rrule("FREQ=DAILY;COUNT=3"),
        ts("2026-11-01T00:00:00Z"),
        ts("2026-12-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 11, 1), date(2026, 11, 2), date(2026, 11, 3)]
    );
}

/// UNTIL includes the occurrence that falls on its date.
#[test]
fn until_is_inclusive() {
    let interval = all_day(date(2026, 11, 1), date(2026, 11, 2));
    let got = occurrences(
        &interval,
        &rrule("FREQ=DAILY;UNTIL=20261103"),
        ts("2026-11-01T00:00:00Z"),
        ts("2026-12-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 11, 1), date(2026, 11, 2), date(2026, 11, 3)]
    );
}

/// INTERVAL, BYDAY, and WKST together choose alternate weeks and days.
#[test]
fn weekly_interval_byday_and_wkst() {
    let interval = all_day(date(2026, 3, 4), date(2026, 3, 5));
    let got = occurrences(
        &interval,
        &rrule("FREQ=WEEKLY;INTERVAL=2;BYDAY=SU,WE;WKST=SU"),
        ts("2026-03-01T00:00:00Z"),
        ts("2026-04-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![
            date(2026, 3, 4),
            date(2026, 3, 15),
            date(2026, 3, 18),
            date(2026, 3, 29),
        ]
    );
}

/// A negative BYMONTHDAY counts back from the month end.
#[test]
fn monthly_negative_month_day() {
    let interval = all_day(date(2026, 1, 31), date(2026, 2, 1));
    let got = occurrences(
        &interval,
        &rrule("FREQ=MONTHLY;BYMONTHDAY=-1"),
        ts("2026-01-01T00:00:00Z"),
        ts("2026-04-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 1, 31), date(2026, 2, 28), date(2026, 3, 31)]
    );
}

/// An ordinal BYDAY selects the last Friday of each month.
#[test]
fn monthly_last_friday() {
    let interval = all_day(date(2026, 1, 30), date(2026, 1, 31));
    let got = occurrences(
        &interval,
        &rrule("FREQ=MONTHLY;BYDAY=-1FR"),
        ts("2026-01-01T00:00:00Z"),
        ts("2026-04-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 1, 30), date(2026, 2, 27), date(2026, 3, 27)]
    );
}

/// YEARLY with BYMONTH and BYMONTHDAY picks one date per year.
#[test]
fn yearly_by_month_and_month_day() {
    let interval = all_day(date(2026, 3, 15), date(2026, 3, 16));
    let got = occurrences(
        &interval,
        &rrule("FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=15"),
        ts("2026-01-01T00:00:00Z"),
        ts("2029-01-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 3, 15), date(2027, 3, 15), date(2028, 3, 15),]
    );
}

/// An ordinal YEARLY BYDAY without BYMONTH is relative to the whole year.
#[test]
fn yearly_last_sunday_of_the_year() {
    let interval = all_day(date(2026, 1, 1), date(2026, 1, 2));
    let got = occurrences(
        &interval,
        &rrule("FREQ=YEARLY;BYDAY=-1SU"),
        ts("2026-01-01T00:00:00Z"),
        ts("2028-01-01T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 1, 1), date(2026, 12, 27), date(2027, 12, 26),]
    );
}

/// The civil conversion round-trips every `When` kind.
#[test]
fn civil_when_round_trips_each_kind() {
    let values = [
        When::Date {
            date: date(2026, 11, 3),
        },
        When::Floating {
            datetime: date(2026, 11, 3).at(9, 0, 0, 0),
        },
        When::Instant {
            timestamp: ts("2026-11-03T15:00:00Z"),
        },
        When::Zoned(Zoned::new(date(2026, 11, 3).at(9, 0, 0, 0), "America/Chicago").unwrap()),
    ];

    for value in values {
        let civil = CivilWhen::from(&value);
        assert_eq!(When::try_from(&civil).unwrap(), value);
    }
}

/// A late window still counts occurrences from DTSTART, not from the window.
#[test]
fn window_after_dtstart_still_honours_count() {
    let interval = all_day(date(2026, 1, 1), date(2026, 1, 2));
    let got = occurrences(
        &interval,
        &rrule("FREQ=DAILY;COUNT=10"),
        ts("2026-01-08T00:00:00Z"),
        ts("2026-01-15T00:00:00Z"),
        &zone("UTC"),
    )
    .unwrap();

    assert_eq!(
        start_dates(&got),
        vec![date(2026, 1, 8), date(2026, 1, 9), date(2026, 1, 10)]
    );
}
