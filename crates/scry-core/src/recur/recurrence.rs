//! RRULE text handling: parse the supported subset into `Recurrence` and
//! render it back canonically. The parser is hand-written so an unsupported part
//! fails loudly instead of being silently ignored and producing wrong dates.

use super::error::{RecurError, invalid};
use jiff::civil::{Date, DateTime, Weekday};
use jiff::tz::TimeZone;
use std::fmt;

/// How often a series repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// One `BYDAY` entry: a weekday plus an optional position within the month or
/// year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByDay {
    /// `None` for a plain `MO`, `Some(-1)` for `-1FR`.
    pub ordinal: Option<i8>,
    pub weekday: Weekday,
}

/// A parsed RRULE. The event stores the text; this typed form exists only while
/// a window is expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recurrence {
    pub freq: Frequency,
    pub interval: u32,
    pub by_day: Vec<ByDay>,
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<i8>,
    pub count: Option<u32>,
    pub until: Option<Until>,
    pub wkst: Weekday,
}

/// The inclusive end of a series: a local date, or an absolute instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    Date(Date),
    Instant(jiff::Timestamp),
}

impl Recurrence {
    /// Parse the supported RRULE subset. An optional `RRULE:` prefix is ignored.
    pub fn parse(rrule: &str) -> Result<Recurrence, RecurError> {
        let body = strip_rrule_prefix(rrule.trim());
        let mut freq = None;
        let mut interval = 1;
        let mut by_day = Vec::new();
        let mut by_month_day = Vec::new();
        let mut by_month = Vec::new();
        let mut count = None;
        let mut until = None;
        let mut wkst = Weekday::Monday;

        for part in body.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let Some((key, value)) = part.split_once('=') else {
                return Err(invalid(part, "expected KEY=VALUE"));
            };
            match key.trim().to_ascii_uppercase().as_str() {
                "FREQ" => freq = Some(parse_frequency(value.trim())?),
                "INTERVAL" => interval = parse_positive(value, "INTERVAL")?,
                "BYDAY" => by_day = parse_by_day(value.trim())?,
                "BYMONTHDAY" => by_month_day = parse_month_days(value.trim())?,
                "BYMONTH" => by_month = parse_months(value.trim())?,
                "COUNT" => count = Some(parse_positive(value, "COUNT")?),
                "UNTIL" => until = Some(parse_until(value)?),
                "WKST" => {
                    let token = value.trim();
                    wkst = parse_weekday(token)
                        .ok_or_else(|| invalid(token, "expected a weekday like MO"))?;
                }
                other => {
                    return Err(RecurError::Unsupported(format!(
                        "unknown RRULE key {other}"
                    )));
                }
            }
        }

        let freq = freq.ok_or_else(|| invalid("FREQ", "missing FREQ"))?;
        if count.is_some() && until.is_some() {
            return Err(RecurError::Unsupported(
                "COUNT and UNTIL are mutually exclusive".to_owned(),
            ));
        }
        if freq == Frequency::Weekly && by_day.iter().any(|day| day.ordinal.is_some()) {
            return Err(RecurError::Unsupported(
                "ordinal BYDAY on a WEEKLY rule".to_owned(),
            ));
        }

        Ok(Recurrence {
            freq,
            interval,
            by_day,
            by_month_day,
            by_month,
            count,
            until,
            wkst,
        })
    }

    /// Render canonical RRULE text, leaving out defaults. Parsing the result
    /// yields an equal rule.
    pub fn to_rrule(&self) -> String {
        let mut parts = vec![format!("FREQ={}", frequency_code(self.freq))];

        if self.interval != 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        if !self.by_month.is_empty() {
            parts.push(format!("BYMONTH={}", join_numbers(&self.by_month)));
        }
        if !self.by_month_day.is_empty() {
            parts.push(format!("BYMONTHDAY={}", join_numbers(&self.by_month_day)));
        }
        if !self.by_day.is_empty() {
            let days: Vec<String> = self
                .by_day
                .iter()
                .map(|day| {
                    let weekday = weekday_code(day.weekday);
                    match day.ordinal {
                        Some(ordinal) => format!("{ordinal}{weekday}"),
                        None => weekday.to_owned(),
                    }
                })
                .collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }
        if self.wkst != Weekday::Monday {
            parts.push(format!("WKST={}", weekday_code(self.wkst)));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        if let Some(until) = &self.until {
            let until = match until {
                Until::Date(date) => date.strftime("%Y%m%d").to_string(),
                Until::Instant(timestamp) => timestamp
                    .to_zoned_datetime(TimeZone::UTC)
                    .datetime()
                    .strftime("%Y%m%dT%H%M%SZ")
                    .to_string(),
            };
            parts.push(format!("UNTIL={until}"));
        }

        parts.join(";")
    }
}

impl fmt::Display for Recurrence {
    /// Format the rule as canonical RRULE text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_rrule())
    }
}

/// The RRULE token for a frequency.
fn frequency_code(frequency: Frequency) -> &'static str {
    match frequency {
        Frequency::Daily => "DAILY",
        Frequency::Weekly => "WEEKLY",
        Frequency::Monthly => "MONTHLY",
        Frequency::Yearly => "YEARLY",
    }
}

/// The two-letter RRULE token for a weekday.
fn weekday_code(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "MO",
        Weekday::Tuesday => "TU",
        Weekday::Wednesday => "WE",
        Weekday::Thursday => "TH",
        Weekday::Friday => "FR",
        Weekday::Saturday => "SA",
        Weekday::Sunday => "SU",
    }
}

/// Join a numeric list in the comma form RRULE expects.
fn join_numbers(values: &[i8]) -> String {
    values
        .iter()
        .map(i8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// Drop an optional `RRULE:` prefix without copying the rest.
fn strip_rrule_prefix(input: &str) -> &str {
    match input.get(..6) {
        Some(prefix) if prefix.eq_ignore_ascii_case("RRULE:") => &input[6..],
        _ => input,
    }
}

/// Parse a `FREQ` value, rejecting frequencies outside the subset.
fn parse_frequency(value: &str) -> Result<Frequency, RecurError> {
    match value.to_ascii_uppercase().as_str() {
        "DAILY" => Ok(Frequency::Daily),
        "WEEKLY" => Ok(Frequency::Weekly),
        "MONTHLY" => Ok(Frequency::Monthly),
        "YEARLY" => Ok(Frequency::Yearly),
        other => Err(RecurError::Unsupported(format!("FREQ {other}"))),
    }
}

/// Parse a two-letter weekday token.
fn parse_weekday(value: &str) -> Option<Weekday> {
    match value.to_ascii_uppercase().as_str() {
        "MO" => Some(Weekday::Monday),
        "TU" => Some(Weekday::Tuesday),
        "WE" => Some(Weekday::Wednesday),
        "TH" => Some(Weekday::Thursday),
        "FR" => Some(Weekday::Friday),
        "SA" => Some(Weekday::Saturday),
        "SU" => Some(Weekday::Sunday),
        _ => None,
    }
}

/// Parse a positive integer for `INTERVAL` or `COUNT`.
fn parse_positive(value: &str, what: &str) -> Result<u32, RecurError> {
    let token = value.trim();
    match token.parse::<u32>() {
        Ok(number) if number > 0 => Ok(number),
        _ => Err(invalid(token, format!("{what} must be a positive integer"))),
    }
}

/// Parse `BYDAY` as plain weekdays or signed ordinals such as `-1FR`.
fn parse_by_day(value: &str) -> Result<Vec<ByDay>, RecurError> {
    let mut days = Vec::new();
    for raw in value.split(',') {
        let token = raw.trim();
        if !token.is_ascii() || token.len() < 2 {
            return Err(invalid(token, "expected a weekday like MO or -1FR"));
        }
        let (prefix, letters) = token.split_at(token.len() - 2);
        let weekday = parse_weekday(letters)
            .ok_or_else(|| invalid(token, "expected a weekday like MO or -1FR"))?;
        let ordinal = if prefix.is_empty() {
            None
        } else {
            let ordinal = prefix
                .parse::<i8>()
                .map_err(|_| invalid(token, "invalid weekday ordinal"))?;
            if ordinal == 0 || !(-53..=53).contains(&ordinal) {
                return Err(invalid(
                    token,
                    "weekday ordinal must be in -53..=53 and not 0",
                ));
            }
            Some(ordinal)
        };
        days.push(ByDay { ordinal, weekday });
    }
    Ok(days)
}

/// Parse `BYMONTHDAY`, allowing negative days counted from the month end.
fn parse_month_days(value: &str) -> Result<Vec<i8>, RecurError> {
    let mut days = Vec::new();
    for raw in value.split(',') {
        let token = raw.trim();
        let day = token
            .parse::<i8>()
            .map_err(|_| invalid(token, "expected a day of month"))?;
        if day == 0 || day.unsigned_abs() > 31 {
            return Err(invalid(token, "day of month must be 1..=31 or -31..=-1"));
        }
        days.push(day);
    }
    Ok(days)
}

/// Parse `BYMONTH` as month numbers in 1..=12.
fn parse_months(value: &str) -> Result<Vec<i8>, RecurError> {
    let mut months = Vec::new();
    for raw in value.split(',') {
        let token = raw.trim();
        let month = token
            .parse::<i8>()
            .map_err(|_| invalid(token, "expected a month"))?;
        if !(1..=12).contains(&month) {
            return Err(invalid(token, "month must be in 1..=12"));
        }
        months.push(month);
    }
    Ok(months)
}

/// Parse `UNTIL` as either a compact date or a compact UTC instant.
fn parse_until(value: &str) -> Result<Until, RecurError> {
    let token = value.trim();
    let upper = token.to_ascii_uppercase();
    if let Some(body) = upper.strip_suffix('Z') {
        if !body.is_ascii() || body.len() != 15 || body.as_bytes().get(8) != Some(&b'T') {
            return Err(invalid(token, "expected YYYYMMDDTHHMMSSZ"));
        }
        let date = parse_compact_date(&body[..8], token)?;
        let time = &body[9..];
        if !time.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid(token, "expected YYYYMMDDTHHMMSSZ"));
        }
        let hour = time[..2]
            .parse::<i8>()
            .map_err(|_| invalid(token, "invalid hour"))?;
        let minute = time[2..4]
            .parse::<i8>()
            .map_err(|_| invalid(token, "invalid minute"))?;
        let second = time[4..6]
            .parse::<i8>()
            .map_err(|_| invalid(token, "invalid second"))?;
        let datetime = DateTime::new(
            date.year(),
            date.month(),
            date.day(),
            hour,
            minute,
            second,
            0,
        )
        .map_err(|error| invalid(token, error.to_string()))?;
        let timestamp = datetime
            .to_zoned_datetime(TimeZone::UTC)
            .map_err(|error| invalid(token, error.to_string()))?
            .timestamp();
        Ok(Until::Instant(timestamp))
    } else {
        Ok(Until::Date(parse_compact_date(&upper, token)?))
    }
}

/// Read `YYYYMMDD`, reporting the original token when it is malformed.
fn parse_compact_date(digits: &str, token: &str) -> Result<Date, RecurError> {
    if digits.len() != 8 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(token, "expected a date as YYYYMMDD"));
    }
    let year = digits[..4]
        .parse::<i16>()
        .map_err(|_| invalid(token, "invalid year"))?;
    let month = digits[4..6]
        .parse::<i8>()
        .map_err(|_| invalid(token, "invalid month"))?;
    let day = digits[6..8]
        .parse::<i8>()
        .map_err(|_| invalid(token, "invalid day"))?;
    Date::new(year, month, day).map_err(|error| invalid(token, error.to_string()))
}
