use jiff::civil::date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use scry_core::models::Event;
use scry_core::recurrence::occurrences;
use scry_core::temporal::{Interval, When, Zoned};
use scry_ics::{IcsError, export, export_events, import};

const ALL_KINDS: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//scry//test//EN\r\n\
BEGIN:VEVENT\r\n\
UID:date@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;VALUE=DATE:20260401\r\n\
DTEND;VALUE=DATE:20260402\r\n\
SUMMARY:All day\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:floating@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART:20260401T120000\r\n\
DTEND:20260401T130000\r\n\
SUMMARY:Floating\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:instant@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART:20260401T120000Z\r\n\
DTEND:20260401T130000Z\r\n\
SUMMARY:Instant\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:zoned@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART;TZID=Europe/Berlin:20260401T120000\r\n\
DTEND;TZID=Europe/Berlin:20260401T130000\r\n\
SUMMARY:Zoned\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

const COUNTING: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//scry//test//EN\r\n\
BEGIN:VEVENT\r\n\
UID:count@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART:20260301T090000Z\r\n\
DTEND:20260301T100000Z\r\n\
RRULE:FREQ=DAILY;COUNT=5\r\n\
EXDATE:20260303T090000Z,20260304T090000Z\r\n\
RDATE:20260310T090000Z\r\n\
SUMMARY:Counted\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:until@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART:20260302T090000Z\r\n\
DTEND:20260302T100000Z\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20260406T130000Z\r\n\
SUMMARY:Until\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

const SPLITTING: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//scry//test//EN\r\n\
BEGIN:VEVENT\r\n\
UID:split@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
DTSTART:20260302T090000Z\r\n\
DTEND:20260302T100000Z\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO\r\n\
SUMMARY:Original\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:split@test\r\n\
DTSTAMP:20260101T000000Z\r\n\
RECURRENCE-ID;RANGE=THISANDFUTURE:20260316T090000Z\r\n\
DTSTART:20260316T110000Z\r\n\
DTEND:20260316T120000Z\r\n\
SUMMARY:Edited\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

#[test]
fn all_four_when_kinds_round_trip() {
    let events = import(ALL_KINDS).unwrap();
    assert_eq!(events.len(), 4);

    let kinds: Vec<&str> = events
        .iter()
        .map(|event| match event.interval.start() {
            When::Date { .. } => "date",
            When::Floating { .. } => "floating",
            When::Instant { .. } => "instant",
            When::Zoned(_) => "zoned",
        })
        .collect();
    assert_eq!(kinds, vec!["date", "floating", "instant", "zoned"]);

    let exported = export(&events).unwrap();
    let reparsed = import(&exported).unwrap();
    assert_eq!(reparsed, events);
}

#[test]
fn a_google_series_keeps_its_single_override() {
    let events = import(include_str!("fixtures/google_weekly.ics")).unwrap();
    assert_eq!(events.len(), 1);

    let series = &events[0];
    let recurrence = series.recurrence.as_ref().unwrap();
    assert_eq!(recurrence.exdates.len(), 1);
    assert_eq!(series.overrides.len(), 1);

    let tz = TimeZone::get("America/New_York").unwrap();
    let from: Timestamp = "2026-01-01T00:00:00Z".parse().unwrap();
    let to: Timestamp = "2026-02-01T00:00:00Z".parse().unwrap();
    let generated = occurrences(&series.interval, recurrence, from, to, &tz).unwrap();
    assert_eq!(generated.len(), 5);

    let original = series.overrides[0]
        .original_start
        .start_instant(&tz)
        .unwrap();
    assert_eq!(original, "2026-01-12T14:00:00Z".parse().unwrap());

    let probe_to = original.checked_add(Span::new().seconds(1)).unwrap();
    let probe = occurrences(&series.interval, recurrence, original, probe_to, &tz).unwrap();
    assert_eq!(probe.len(), 1);
    assert_eq!(probe[0].start_instant(&tz).unwrap(), original);
}

#[test]
fn a_weekly_series_keeps_its_local_wall_clock_across_dst() {
    let events = import(include_str!("fixtures/outlook_dst.ics")).unwrap();
    assert_eq!(events.len(), 1);

    let series = &events[0];
    match series.interval.start() {
        When::Zoned(zoned) => {
            assert_eq!(zoned.zone(), "America/New_York");
            assert_eq!(zoned.datetime(), date(2026, 2, 23).at(9, 0, 0, 0));
        }
        other => panic!("expected a zoned start, got {other:?}"),
    }

    let tz = TimeZone::get("America/New_York").unwrap();
    let from: Timestamp = "2026-02-01T00:00:00Z".parse().unwrap();
    let to: Timestamp = "2026-04-08T00:00:00Z".parse().unwrap();
    let expanded = occurrences(
        &series.interval,
        series.recurrence.as_ref().unwrap(),
        from,
        to,
        &tz,
    )
    .unwrap();

    for interval in &expanded {
        match interval.start() {
            When::Zoned(zoned) => assert_eq!(zoned.datetime().hour(), 9),
            other => panic!("expected a zoned start, got {other:?}"),
        }
    }

    let starts: Vec<Timestamp> = expanded
        .iter()
        .map(|interval| interval.start_instant(&tz).unwrap())
        .collect();
    assert!(starts.contains(&"2026-03-02T14:00:00Z".parse().unwrap()));
    assert!(starts.contains(&"2026-03-09T13:00:00Z".parse().unwrap()));
}

#[test]
fn counts_exdates_and_rdates_round_trip() {
    let events = import(COUNTING).unwrap();
    assert_eq!(events.len(), 2);

    let counted = events[0].recurrence.as_ref().unwrap();
    assert!(counted.text.contains("COUNT=5"));
    assert_eq!(counted.exdates.len(), 2);
    assert_eq!(counted.rdates.len(), 1);

    let until = events[1].recurrence.as_ref().unwrap();
    assert!(until.text.contains("UNTIL=20260406T130000Z"));

    let exported = export(&events).unwrap();
    let reparsed = import(&exported).unwrap();
    assert_eq!(reparsed, events);
}

#[test]
fn an_unknown_tzid_is_reported_by_name() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:nowhere\r\n\
DTSTAMP:20260101T000000Z\r\nDTSTART;TZID=Custom/Nowhere:20260401T120000\r\n\
DTEND;TZID=Custom/Nowhere:20260401T130000\r\nSUMMARY:Nowhere\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    match import(input) {
        Err(IcsError::UnknownTimezone(name)) => assert_eq!(name, "Custom/Nowhere"),
        other => panic!("expected an unknown time zone error, got {other:?}"),
    }
}

#[test]
fn a_malformed_rule_is_reported_as_invalid() {
    for rule in ["FREQ=WEEKLY;BYDAY=XX", "FREQ=DAILY;COUNT=abc"] {
        let input = format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:broken\r\n\
DTSTAMP:20260101T000000Z\r\nDTSTART:20260401T120000Z\r\nDTEND:20260401T130000Z\r\n\
RRULE:{rule}\r\nSUMMARY:Broken\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        );
        match import(&input) {
            Err(IcsError::InvalidRrule(text)) => assert_eq!(text, rule),
            other => panic!("expected an invalid rule error for {rule}, got {other:?}"),
        }
    }
}

#[test]
fn a_folded_rule_is_unfolded_before_validation() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:folded\r\n\
DTSTAMP:20260101T000000Z\r\nDTSTART:20260401T120000Z\r\nDTEND:20260401T130000Z\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,\r\n TU\r\nSUMMARY:Folded\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    let events = import(input).unwrap();
    assert_eq!(
        events[0].recurrence.as_ref().unwrap().text,
        "FREQ=WEEKLY;BYDAY=MO,TU"
    );
}

#[test]
fn an_unsupported_rule_is_not_reported_as_invalid() {
    let input = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:unsupported\r\n\
DTSTAMP:20260101T000000Z\r\nDTSTART:20260401T120000Z\r\nDTEND:20260401T130000Z\r\n\
RRULE:FREQ=MONTHLY;BYSETPOS=-1\r\nSUMMARY:Unsupported\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    match import(input) {
        Err(IcsError::UnsupportedRrule(text)) => assert!(text.contains("BYSETPOS")),
        other => panic!("expected an unsupported rule error, got {other:?}"),
    }
}

#[test]
fn folded_and_escaped_text_survives() {
    let events = import(include_str!("fixtures/google_weekly.ics")).unwrap();
    let series = &events[0];

    assert_eq!(series.title, "Team sync, weekly planning and follow-ups");
    assert_eq!(
        series.description.as_deref(),
        Some("Agenda: roadmap, staffing; next steps\nBring notes.")
    );
}

#[test]
fn unknown_properties_are_preserved() {
    let events = import(include_str!("fixtures/apple_allday.ics")).unwrap();
    let allday = events
        .iter()
        .find(|event| event.external_metadata.uid.as_deref() == Some("apple-allday@example.com"))
        .unwrap();

    let color = allday
        .external_metadata
        .extra
        .iter()
        .find(|property| property.name == "X-APPLE-CALENDAR-COLOR")
        .unwrap();
    assert_eq!(color.value, "#FF2968");

    let exported = export(&events).unwrap();
    let reparsed = import(&exported).unwrap();
    let allday_again = reparsed
        .iter()
        .find(|event| event.external_metadata.uid.as_deref() == Some("apple-allday@example.com"))
        .unwrap();
    assert_eq!(
        allday_again.external_metadata.extra,
        allday.external_metadata.extra
    );
}

#[test]
fn a_this_and_future_override_splits_the_series() {
    let events = import(SPLITTING).unwrap();
    assert_eq!(events.len(), 2);

    assert_eq!(events[0].title, "Original");
    assert_eq!(
        events[0].recurrence.as_ref().unwrap().text,
        "FREQ=WEEKLY;BYDAY=MO;UNTIL=20260309T090000Z"
    );

    assert_eq!(events[1].title, "Edited");
    assert_eq!(
        events[1].external_metadata.uid.as_deref(),
        Some("split@test#1")
    );
    assert_eq!(
        events[1].recurrence.as_ref().unwrap().text,
        "FREQ=WEEKLY;BYDAY=MO"
    );
    assert_eq!(
        events[1].interval.start(),
        &When::Instant {
            timestamp: "2026-03-16T11:00:00Z".parse().unwrap(),
        }
    );
}

#[test]
fn export_events_writes_a_parseable_calendar() {
    let interval = Interval::new(
        When::Zoned(Zoned::new(date(2026, 5, 1).at(9, 0, 0, 0), "America/Chicago").unwrap()),
        When::Zoned(Zoned::new(date(2026, 5, 1).at(10, 0, 0, 0), "America/Chicago").unwrap()),
    )
    .unwrap();

    let event = Event {
        id: Default::default(),
        project_id: Default::default(),
        title: "Plain event".to_owned(),
        description: None,
        interval,
        recurrence: None,
        overrides: Vec::new(),
        external_metadata: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    };

    let exported = export_events(&[event]).unwrap();
    for property in ["UID:", "DTSTAMP:", "DTSTART", "DTEND"] {
        assert!(
            exported.contains(property),
            "missing {property} in:\n{exported}"
        );
    }

    let reparsed = import(&exported).unwrap();
    assert_eq!(reparsed.len(), 1);
    assert_eq!(reparsed[0].title, "Plain event");
}
