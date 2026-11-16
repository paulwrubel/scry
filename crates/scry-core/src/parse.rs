//! A deliberately small, hand-written date/time grammar for CLI input such as
//! `scry add --due "fri 5pm"`. The CLI needs readable input, not natural
//! language, so only the forms documented on [`parse_when`] are understood. A
//! missing zone means the caller's zone, while an explicit zone token wins.

use crate::temporal::{When, Zoned};
use jiff::Span;
use jiff::Timestamp;
use jiff::civil::{self, Weekday};
use jiff::tz::TimeZone;
use std::fmt;

/// Parse one of the small human date/time forms accepted by the CLI.
///
/// This is deliberately a hand-written subset rather than a natural-language
/// parser: the CLI needs readable input, not arbitrary prose. `now` and `zone`
/// supply the caller's local date, and `floating` makes a time-bearing value
/// float instead of being tied to `zone`. A trailing IANA zone token always
/// wins over both.
///
/// Accepted forms are `today`, `tomorrow`, `yesterday`, weekday names, ISO
/// dates and datetimes, and times such as `5pm` or `17:00`. A date may be
/// followed by a time, and any time-bearing form may be followed by a zone.
pub fn parse_when(
    input: &str,
    now: Timestamp,
    zone: &str,
    floating: bool,
) -> Result<When, ParseError> {
    let caller_tz = TimeZone::get(zone)
        .map_err(|_| ParseError::new(format!("invalid IANA time zone: {zone:?}")))?;
    let local = now.to_zoned_datetime(caller_tz);
    let today = local.date();

    let tokens: Vec<&str> = input.split_whitespace().collect();
    let Some(&first) = tokens.first() else {
        return Err(ParseError::new("empty date/time input"));
    };

    let (date, mut index) = if let Some(date) = parse_keyword_date(first, today)? {
        (date, 1)
    } else if let Some(weekday) = parse_weekday(first) {
        (weekday_date(today, weekday)?, 1)
    } else if is_iso_datetime(first) {
        let (datetime, instant) = parse_iso_datetime(first)?;
        return finish_datetime(datetime, instant, &tokens[1..], zone, floating);
    } else if is_iso_date(first) {
        let date = first
            .parse::<civil::Date>()
            .map_err(|error| ParseError::new(format!("invalid date {first:?}: {error}")))?;
        (date, 1)
    } else if looks_like_time(first) {
        let time = parse_time(first)?;
        return finish_datetime(today.to_datetime(time), None, &tokens[1..], zone, floating);
    } else {
        return Err(ParseError::new(format!(
            "unknown date/time token: {first:?}"
        )));
    };

    let mut datetime = None;
    if let Some(token) = tokens
        .get(index)
        .copied()
        .filter(|token| looks_like_time(token))
    {
        let time = parse_time(token)?;
        datetime = Some(date.to_datetime(time));
        index += 1;
    }

    match datetime {
        Some(datetime) => finish_datetime(datetime, None, &tokens[index..], zone, floating),
        None if index < tokens.len() => Err(ParseError::new(format!(
            "trailing tokens: {}",
            tokens[index..].join(" ")
        ))),
        None => Ok(When::Date { date }),
    }
}

fn parse_keyword_date(token: &str, today: civil::Date) -> Result<Option<civil::Date>, ParseError> {
    let date = match token.to_ascii_lowercase().as_str() {
        "today" => today,
        "tomorrow" => today
            .tomorrow()
            .map_err(|error| ParseError::new(format!("invalid date: {error}")))?,
        "yesterday" => today
            .yesterday()
            .map_err(|error| ParseError::new(format!("invalid date: {error}")))?,
        _ => return Ok(None),
    };
    Ok(Some(date))
}

fn parse_weekday(token: &str) -> Option<Weekday> {
    match token.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Monday),
        "tue" | "tuesday" => Some(Weekday::Tuesday),
        "wed" | "wednesday" => Some(Weekday::Wednesday),
        "thu" | "thursday" => Some(Weekday::Thursday),
        "fri" | "friday" => Some(Weekday::Friday),
        "sat" | "saturday" => Some(Weekday::Saturday),
        "sun" | "sunday" => Some(Weekday::Sunday),
        _ => None,
    }
}

fn weekday_date(today: civil::Date, weekday: Weekday) -> Result<civil::Date, ParseError> {
    let delta = (weekday.to_monday_zero_offset() - today.weekday().to_monday_zero_offset())
        .rem_euclid(7) as i64;
    today
        .checked_add(Span::new().days(delta))
        .map_err(|error| ParseError::new(format!("invalid weekday date: {error}")))
}

fn is_iso_date(token: &str) -> bool {
    token.len() == 10 && is_iso_date_shape(token)
}

fn is_iso_datetime(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() > 10 && bytes[10] == b'T' && is_iso_date_shape(&token[..10])
}

fn is_iso_date_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn parse_iso_datetime(token: &str) -> Result<(civil::DateTime, Option<Timestamp>), ParseError> {
    let rest = &token[11..];
    let offset_at = rest.find(['Z', 'z', '+', '-']);
    let (datetime_text, offset_text) = match offset_at {
        Some(offset) => (&token[..11 + offset], Some(&token[11 + offset..])),
        None => (token, None),
    };

    let datetime = datetime_text
        .parse::<civil::DateTime>()
        .map_err(|error| ParseError::new(format!("invalid ISO datetime {token:?}: {error}")))?;

    let instant = match offset_text {
        None => None,
        Some(offset) => {
            let normalized = if offset.eq_ignore_ascii_case("z") {
                format!("{datetime_text}Z")
            } else {
                format!("{datetime_text}{offset}")
            };
            Some(normalized.parse::<Timestamp>().map_err(|error| {
                ParseError::new(format!("invalid ISO datetime {token:?}: {error}"))
            })?)
        }
    };

    Ok((datetime, instant))
}

fn looks_like_time(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    lower.ends_with("am") || lower.ends_with("pm") || token.contains(':')
}

fn parse_time(token: &str) -> Result<civil::Time, ParseError> {
    let lower = token.to_ascii_lowercase();
    let (body, meridiem) = if let Some(body) = lower.strip_suffix("am") {
        (body, Some(false))
    } else if let Some(body) = lower.strip_suffix("pm") {
        (body, Some(true))
    } else {
        (lower.as_str(), None)
    };

    let mut parts = body.split(':');
    let hour_part = parts.next().unwrap_or("");
    let hour = parse_time_component(hour_part, 1, 2, "hour", token)?;
    let minute = match parts.next() {
        Some(part) => parse_time_component(part, 2, 2, "minute", token)?,
        None => 0,
    };
    let second = match parts.next() {
        Some(part) => parse_time_component(part, 2, 2, "second", token)?,
        None => 0,
    };
    if parts.next().is_some() {
        return Err(ParseError::new(format!("invalid time {token:?}")));
    }

    let hour = match meridiem {
        Some(is_pm) => {
            if !(1..=12).contains(&hour) {
                return Err(ParseError::new(format!(
                    "invalid 12-hour time {token:?}: hour must be 1 through 12"
                )));
            }
            let hour = if hour == 12 { 0 } else { hour };
            if is_pm { hour + 12 } else { hour }
        }
        None => {
            if hour > 23 {
                return Err(ParseError::new(format!(
                    "invalid 24-hour time {token:?}: hour must be 0 through 23"
                )));
            }
            hour
        }
    };

    civil::Time::new(hour as i8, minute as i8, second as i8, 0)
        .map_err(|error| ParseError::new(format!("invalid time {token:?}: {error}")))
}

fn parse_time_component(
    text: &str,
    min_len: usize,
    max_len: usize,
    name: &str,
    token: &str,
) -> Result<u32, ParseError> {
    if text.len() < min_len
        || text.len() > max_len
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ParseError::new(format!("invalid {name} in time {token:?}")));
    }
    text.parse::<u32>()
        .map_err(|error| ParseError::new(format!("invalid {name} in time {token:?}: {error}")))
}

fn finish_datetime(
    datetime: civil::DateTime,
    instant: Option<Timestamp>,
    remaining: &[&str],
    caller_zone: &str,
    floating: bool,
) -> Result<When, ParseError> {
    if remaining.is_empty() {
        return match instant {
            Some(timestamp) => Ok(When::Instant { timestamp }),
            None => finish_time(datetime, None, caller_zone, floating),
        };
    }
    if remaining.len() == 1 {
        let zone = remaining[0];
        if !looks_like_time(zone) {
            return finish_time(datetime, Some(zone), caller_zone, floating);
        }
    }
    Err(ParseError::new(format!(
        "trailing tokens: {}",
        remaining.join(" ")
    )))
}

fn finish_time(
    datetime: civil::DateTime,
    explicit_zone: Option<&str>,
    caller_zone: &str,
    floating: bool,
) -> Result<When, ParseError> {
    if let Some(zone) = explicit_zone {
        let zoned = Zoned::new(datetime, zone)
            .map_err(|_| ParseError::new(format!("invalid IANA time zone: {zone:?}")))?;
        return Ok(When::Zoned(zoned));
    }
    if floating {
        return Ok(When::Floating { datetime });
    }
    let zoned = Zoned::new(datetime, caller_zone)
        .map_err(|_| ParseError::new(format!("invalid IANA time zone: {caller_zone:?}")))?;
    Ok(When::Zoned(zoned))
}

/// A rejected date/time expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn now() -> Timestamp {
        "2026-11-06T15:00:00Z".parse().unwrap()
    }

    fn parse(input: &str, floating: bool) -> Result<When, ParseError> {
        parse_when(input, now(), "America/Chicago", floating)
    }

    fn date_when(year: i16, month: i8, day: i8) -> When {
        When::Date {
            date: date(year, month, day),
        }
    }

    fn zoned_when(year: i16, month: i8, day: i8, hour: i8, minute: i8, zone: &str) -> When {
        When::Zoned(Zoned::new(date(year, month, day).at(hour, minute, 0, 0), zone).unwrap())
    }

    fn instant_when(timestamp: &str) -> When {
        When::Instant {
            timestamp: timestamp.parse().unwrap(),
        }
    }

    #[test]
    fn parses_supported_forms() {
        let cases = [
            ("today", false, date_when(2026, 11, 6)),
            ("tomorrow", false, date_when(2026, 11, 7)),
            ("yesterday", false, date_when(2026, 11, 5)),
            ("fri", false, date_when(2026, 11, 6)),
            ("sat", false, date_when(2026, 11, 7)),
            ("2026-11-03", false, date_when(2026, 11, 3)),
            (
                "2026-11-03T09:00",
                false,
                zoned_when(2026, 11, 3, 9, 0, "America/Chicago"),
            ),
            (
                "2026-11-03T09:00Z",
                false,
                instant_when("2026-11-03T09:00:00Z"),
            ),
            (
                "2026-11-03T09:00-06:00",
                false,
                instant_when("2026-11-03T15:00:00Z"),
            ),
            (
                "5pm",
                false,
                zoned_when(2026, 11, 6, 17, 0, "America/Chicago"),
            ),
            (
                "17:00",
                false,
                zoned_when(2026, 11, 6, 17, 0, "America/Chicago"),
            ),
            (
                "fri 5pm America/Chicago",
                false,
                zoned_when(2026, 11, 6, 17, 0, "America/Chicago"),
            ),
            (
                "today 5pm",
                true,
                When::Floating {
                    datetime: date(2026, 11, 6).at(17, 0, 0, 0),
                },
            ),
            ("2028-02-29", false, date_when(2028, 2, 29)),
            (
                "2026-03-08 02:30 America/Chicago",
                false,
                zoned_when(2026, 3, 8, 2, 30, "America/Chicago"),
            ),
        ];

        for (input, floating, expected) in cases {
            assert_eq!(parse(input, floating).unwrap(), expected, "input {input:?}");
        }
    }

    #[test]
    fn rejects_invalid_forms_with_messages() {
        let cases = [
            ("banana", "banana"),
            ("2026-02-30", "2026-02-30"),
            ("5pm Mars/Olympus", "Mars/Olympus"),
            ("fri banana", "banana"),
        ];

        for (input, mention) in cases {
            let error = parse(input, false).unwrap_err();
            assert!(
                error.to_string().contains(mention),
                "input {input:?} produced {error:?}"
            );
        }

        assert!(parse("", false).is_err());
    }
}
