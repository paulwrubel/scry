//! RRULE handling. calcard owns the RFC grammar and the canonical text, while
//! scry's own parser decides whether its expander can represent the rule.

use crate::error::IcsError;
use calcard::icalendar::{
    ICalendarDay, ICalendarFrequency, ICalendarMonth, ICalendarRecurrenceRule, ICalendarWeekday,
};
use jiff::civil::Weekday;
use scry_core::recurrence::{Frequency, RRule, Recurrence, Until};
use scry_core::temporal::When;

/// Build the stored rule from calcard's typed form. A rule the parser accepts
/// but scry's expander rejects is reported as unsupported, not invalid.
pub(crate) fn recurrence_from_rule(
    rule: &ICalendarRecurrenceRule,
    start: &When,
    exdates: Vec<When>,
    rdates: Vec<When>,
) -> Result<RRule, IcsError> {
    let text = rule.to_string();
    let rrule = RRule {
        text: text.clone(),
        exdates,
        rdates,
    };
    rrule
        .validate(start)
        .map_err(|_| IcsError::UnsupportedRrule(text))?;
    Ok(rrule)
}

/// Render a stored rule back into calcard's typed form, leaving defaults out so
/// the text stays canonical.
pub(crate) fn rule_from_recurrence(
    rule: &RRule,
    _start: &When,
) -> Result<ICalendarRecurrenceRule, IcsError> {
    let text = rule.text.as_str();
    let recurrence = Recurrence::parse(text)
        .map_err(|error| IcsError::InvalidRrule(format!("{text}: {error}")))?;
    let interval = u16::try_from(recurrence.interval)
        .map_err(|_| IcsError::UnsupportedRrule(rule.text.clone()))?;

    Ok(ICalendarRecurrenceRule {
        freq: frequency_from(recurrence.freq),
        interval: (interval != 1).then_some(interval),
        count: recurrence.count,
        until: recurrence.until.map(until_partial),
        byday: recurrence
            .by_day
            .iter()
            .map(|day| ICalendarDay {
                ordwk: day.ordinal.map(i16::from),
                weekday: weekday_from(day.weekday),
            })
            .collect(),
        bymonthday: recurrence.by_month_day.clone(),
        bymonth: recurrence
            .by_month
            .iter()
            .map(|month| ICalendarMonth::new(*month as u8, false))
            .collect(),
        wkst: (recurrence.wkst != Weekday::Monday).then(|| weekday_from(recurrence.wkst)),
        ..Default::default()
    })
}

fn frequency_from(frequency: Frequency) -> ICalendarFrequency {
    match frequency {
        Frequency::Daily => ICalendarFrequency::Daily,
        Frequency::Weekly => ICalendarFrequency::Weekly,
        Frequency::Monthly => ICalendarFrequency::Monthly,
        Frequency::Yearly => ICalendarFrequency::Yearly,
    }
}

fn weekday_from(weekday: Weekday) -> ICalendarWeekday {
    match weekday {
        Weekday::Monday => ICalendarWeekday::Monday,
        Weekday::Tuesday => ICalendarWeekday::Tuesday,
        Weekday::Wednesday => ICalendarWeekday::Wednesday,
        Weekday::Thursday => ICalendarWeekday::Thursday,
        Weekday::Friday => ICalendarWeekday::Friday,
        Weekday::Saturday => ICalendarWeekday::Saturday,
        Weekday::Sunday => ICalendarWeekday::Sunday,
    }
}

fn until_partial(until: Until) -> calcard::common::PartialDateTime {
    match until {
        Until::Date(date) => calcard::common::PartialDateTime {
            year: Some(date.year() as u16),
            month: Some(date.month() as u8),
            day: Some(date.day() as u8),
            ..Default::default()
        },
        Until::Instant(timestamp) => crate::temporal::partial_from_timestamp(timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    use scry_core::temporal::Zoned;

    fn zoned_start() -> When {
        When::Zoned(Zoned::new(date(2026, 1, 5).at(9, 0, 0, 0), "America/New_York").unwrap())
    }

    #[test]
    fn canonical_rules_round_trip() {
        let start = zoned_start();

        for text in [
            "FREQ=DAILY",
            "FREQ=DAILY;INTERVAL=3",
            "FREQ=WEEKLY;BYDAY=MO,WE,FR",
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU;WKST=SU",
            "FREQ=MONTHLY;BYMONTHDAY=1,15",
            "FREQ=MONTHLY;BYDAY=-1FR",
            "FREQ=YEARLY;BYMONTH=3;BYDAY=2SU",
            "FREQ=DAILY;COUNT=10",
            "FREQ=DAILY;UNTIL=20261231",
            "FREQ=DAILY;UNTIL=20261231T235959Z",
        ] {
            let parsed = Recurrence::parse(text).unwrap();
            let stored = RRule {
                text: parsed.to_rrule(),
                exdates: Vec::new(),
                rdates: Vec::new(),
            };
            let calendar_rule = rule_from_recurrence(&stored, &start).unwrap();
            let rendered =
                recurrence_from_rule(&calendar_rule, &start, Vec::new(), Vec::new()).unwrap();
            let reparsed = rendered.validate(&start).unwrap();
            assert_eq!(reparsed, parsed, "round trip failed for {text}");
        }
    }
}
