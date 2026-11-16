//! Short human labels for [`When`] values, shared by the CLI and web UI.

use crate::temporal::{Interval, When};
use jiff::Timestamp;
use jiff::civil;
use jiff::tz::TimeZone;

/// Format `when` as the short label shown in the CLI and web UI.
///
/// Relative labels stop at a week because a label like `in 23d` reads worse
/// than the date it stands for. The zone label only appears when a zoned
/// value's zone differs from the viewer's own, since repeating the viewer's
/// zone on every row is noise.
pub fn format_when(when: &When, now: Timestamp, zone: &TimeZone) -> String {
    let today = now.to_zoned_datetime(zone.clone()).date();
    let (date, time) = wall(when, zone);

    let mut label = day_part(date, today);
    if let Some(time) = time {
        label.push(' ');
        label.push_str(&time_label(time));
    }

    label.push_str(&when_tag(when, zone));
    label
}

/// Format an event's start and end as one short label.
///
/// A timed value on one day shows its end as a time only, so the common case
/// reads `today 10:00 AM – 11:00 AM`. An all-day value shows its inclusive last
/// day, because the stored end is exclusive.
pub fn format_interval(interval: &Interval, now: Timestamp, zone: &TimeZone) -> String {
    let today = now.to_zoned_datetime(zone.clone()).date();
    let (start_date, start_time) = wall(interval.start(), zone);

    let Some(start_time) = start_time else {
        let end = interval.inclusive_end_date().unwrap_or(start_date);
        let label = day_part(start_date, today);
        if end == start_date {
            return label;
        }
        return format!("{label} – {}", absolute_date(end, today));
    };

    let (end_date, end_time) = wall(interval.end(), zone);
    let end_time = end_time.unwrap_or(start_time);
    let mut label = format!("{} {}", day_part(start_date, today), time_label(start_time));
    if end_date == start_date {
        label.push_str(&format!(" – {}", time_label(end_time)));
    } else {
        label.push_str(&format!(
            " – {} {}",
            day_part(end_date, today),
            time_label(end_time)
        ));
    }

    label.push_str(&when_tag(interval.start(), zone));
    label
}

fn wall(when: &When, zone: &TimeZone) -> (civil::Date, Option<civil::Time>) {
    match when {
        When::Date { date } => (*date, None),
        When::Floating { datetime } => (datetime.date(), Some(datetime.time())),
        When::Zoned(zoned) => (zoned.datetime().date(), Some(zoned.datetime().time())),
        When::Instant { timestamp } => {
            let zoned = timestamp.to_zoned_datetime(zone.clone());
            (zoned.date(), Some(zoned.time()))
        }
    }
}

fn time_label(time: civil::Time) -> String {
    time.strftime("%-I:%M %p").to_string()
}

fn when_tag(when: &When, zone: &TimeZone) -> String {
    match when {
        When::Zoned(zoned) => {
            if zone
                .iana_name()
                .is_some_and(|viewer| zoned.zone() != viewer)
            {
                format!(" ({})", zoned.zone())
            } else {
                String::new()
            }
        }
        When::Floating { .. } => " (floating)".to_string(),
        _ => String::new(),
    }
}

fn day_part(date: civil::Date, today: civil::Date) -> String {
    match date.since(today) {
        Ok(span) => match span.get_days() {
            -1 => "yesterday".to_string(),
            0 => "today".to_string(),
            1 => "tomorrow".to_string(),
            days @ 2..=6 => format!("in {days}d"),
            _ => absolute_date(date, today),
        },
        Err(_) => absolute_date(date, today),
    }
}

fn absolute_date(date: civil::Date, today: civil::Date) -> String {
    if date.year() == today.year() {
        date.strftime("%b %-d").to_string()
    } else {
        date.strftime("%b %-d, %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::Zoned;
    use jiff::civil::date;

    fn now() -> Timestamp {
        "2026-11-06T15:00:00Z".parse().unwrap()
    }

    fn zone(name: &str) -> TimeZone {
        TimeZone::get(name).unwrap()
    }

    fn date_when(year: i16, month: i8, day: i8) -> When {
        When::Date {
            date: date(year, month, day),
        }
    }

    fn zoned_when(day: i8, hour: i8, zone: &str) -> When {
        When::Zoned(Zoned::new(date(2026, 11, day).at(hour, 0, 0, 0), zone).unwrap())
    }

    fn format(when: &When, viewer: &TimeZone) -> String {
        format_when(when, now(), viewer)
    }

    #[test]
    fn date_today_and_near_future_are_relative() {
        let chicago = zone("America/Chicago");
        assert_eq!(format(&date_when(2026, 11, 6), &chicago), "today");
        assert_eq!(format(&date_when(2026, 11, 9), &chicago), "in 3d");
    }

    #[test]
    fn yesterday_is_relative() {
        assert_eq!(
            format(&date_when(2026, 11, 5), &zone("America/Chicago")),
            "yesterday"
        );
    }

    #[test]
    fn dates_outside_the_window_are_absolute() {
        let chicago = zone("America/Chicago");
        assert_eq!(format(&date_when(2026, 11, 3), &chicago), "Nov 3");
        assert_eq!(format(&date_when(2027, 1, 2), &chicago), "Jan 2, 2027");
    }

    #[test]
    fn zoned_value_in_the_viewers_zone_has_no_label() {
        let chicago = zone("America/Chicago");
        let when = zoned_when(6, 17, "America/Chicago");
        assert_eq!(format(&when, &chicago), "today 5:00 PM");
    }

    #[test]
    fn zoned_value_in_another_zone_is_labelled() {
        let when = zoned_when(6, 17, "America/Chicago");
        assert_eq!(
            format(&when, &zone("America/New_York")),
            "today 5:00 PM (America/Chicago)"
        );
    }

    #[test]
    fn instant_is_displayed_in_the_viewers_zone_without_a_label() {
        let instant = When::Instant {
            timestamp: "2026-11-06T23:00:00Z".parse().unwrap(),
        };
        assert_eq!(format(&instant, &zone("America/Chicago")), "today 5:00 PM");
        assert_eq!(format(&instant, &zone("America/New_York")), "today 6:00 PM");
    }

    #[test]
    fn floating_value_carries_a_marker() {
        let floating = When::Floating {
            datetime: date(2026, 11, 6).at(17, 0, 0, 0),
        };
        assert_eq!(
            format(&floating, &zone("America/Chicago")),
            "today 5:00 PM (floating)"
        );
    }

    #[test]
    fn all_day_interval_shows_its_inclusive_last_day() {
        let chicago = zone("America/Chicago");
        let one_day = Interval::new(date_when(2026, 11, 6), date_when(2026, 11, 7)).unwrap();
        assert_eq!(format_interval(&one_day, now(), &chicago), "today");

        let three_days = Interval::new(date_when(2026, 11, 6), date_when(2026, 11, 9)).unwrap();
        assert_eq!(
            format_interval(&three_days, now(), &chicago),
            "today – Nov 8"
        );
    }

    #[test]
    fn timed_interval_on_one_day_shows_only_the_end_time() {
        let chicago = zone("America/Chicago");
        let interval = Interval::new(
            zoned_when(6, 10, "America/Chicago"),
            zoned_when(6, 11, "America/Chicago"),
        )
        .unwrap();
        assert_eq!(
            format_interval(&interval, now(), &chicago),
            "today 10:00 AM – 11:00 AM"
        );
    }

    #[test]
    fn timed_interval_across_days_shows_both_days() {
        let chicago = zone("America/Chicago");
        let interval = Interval::new(
            zoned_when(6, 22, "America/Chicago"),
            zoned_when(7, 9, "America/Chicago"),
        )
        .unwrap();
        assert_eq!(
            format_interval(&interval, now(), &chicago),
            "today 10:00 PM – tomorrow 9:00 AM"
        );
    }

    #[test]
    fn foreign_zone_is_labelled_once() {
        let interval = Interval::new(
            zoned_when(6, 10, "America/Chicago"),
            zoned_when(6, 11, "America/Chicago"),
        )
        .unwrap();
        assert_eq!(
            format_interval(&interval, now(), &zone("America/New_York")),
            "today 10:00 AM – 11:00 AM (America/Chicago)"
        );
    }
}
