//! Short human labels for [`When`] values, shared by the CLI and web UI.

use crate::temporal::When;
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

    let (date, time) = match when {
        When::Date { date } => (*date, None),
        When::Floating { datetime } => (datetime.date(), Some(datetime.time())),
        When::Zoned(zoned) => {
            let datetime = zoned.datetime();
            (datetime.date(), Some(datetime.time()))
        }
        When::Instant { timestamp } => {
            let zoned = timestamp.to_zoned_datetime(zone.clone());
            (zoned.date(), Some(zoned.time()))
        }
    };

    let mut label = day_part(date, today);
    if let Some(time) = time {
        label.push(' ');
        label.push_str(&time.strftime("%-I:%M %p").to_string());
    }

    match when {
        When::Zoned(zoned) => {
            if zone
                .iana_name()
                .is_some_and(|viewer| zoned.zone() != viewer)
            {
                label.push_str(&format!(" ({})", zoned.zone()));
            }
        }
        When::Floating { .. } => label.push_str(" (floating)"),
        _ => {}
    }

    label
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
}
