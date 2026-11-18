//! Shared helpers for the parser and expansion tests.

mod expand;
mod parse;

use super::*;
use crate::temporal::{Interval, When};
use jiff::Timestamp;
use jiff::civil::{Date, date};
use jiff::tz::TimeZone;

/// Resolve a zone used by an expansion test.
fn zone(name: &str) -> TimeZone {
    TimeZone::get(name).unwrap()
}

/// Parse an instant literal.
fn ts(value: &str) -> Timestamp {
    value.parse().unwrap()
}

/// A stored rule with no added or excluded occurrences.
fn rrule(text: &str) -> RRule {
    RRule {
        text: text.to_owned(),
        exdates: Vec::new(),
        rdates: Vec::new(),
    }
}

/// A whole-day interval, whose end is the next midnight.
fn all_day(start: Date, end: Date) -> Interval {
    Interval::new(When::Date { date: start }, When::Date { date: end }).unwrap()
}

/// The start dates of date intervals, for compact assertions.
fn start_dates(intervals: &[Interval]) -> Vec<Date> {
    intervals
        .iter()
        .map(|interval| match interval.start() {
            When::Date { date } => *date,
            other => panic!("unexpected start {other:?}"),
        })
        .collect()
}
