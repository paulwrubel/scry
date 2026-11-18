//! Conversions between calcard's iCalendar values and scry's `When`. Every
//! chrono and jiff type conversion lives here, so the rest of the crate only
//! sees jiff civil values and `When`.

use crate::error::IcsError;
use calcard::common::PartialDateTime;
use calcard::common::timezone::Tz;
use calcard::icalendar::timezone::TzResolver;
use calcard::icalendar::{
    ICalendarDuration, ICalendarEntry, ICalendarParameter, ICalendarParameterName,
    ICalendarProperty, ICalendarValue, ICalendarValueType,
};
use jiff::civil;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use scry_core::temporal::{DEFAULT_EVENT_MINUTES, Interval, When, Zoned};

/// Read the first date or time value in `entry`. The caller uses this when a
/// property must have exactly one value, such as DTSTART or RECURRENCE-ID.
pub(crate) fn when_from_entry(
    entry: &ICalendarEntry,
    resolver: &TzResolver<&str>,
) -> Result<When, IcsError> {
    whens_from_entry(entry, resolver)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            IcsError::InvalidTemporal(format!("{} has no date or time value", entry.name.as_str()))
        })
}

/// Read every date or time value in `entry`, which is how a multi-valued
/// EXDATE or RDATE keeps all of its starts.
pub(crate) fn whens_from_entry(
    entry: &ICalendarEntry,
    resolver: &TzResolver<&str>,
) -> Result<Vec<When>, IcsError> {
    let zone = resolved_zone(entry, resolver)?;
    let mut whens = Vec::new();
    for value in &entry.values {
        if let ICalendarValue::PartialDateTime(datetime) = value {
            whens.push(when_from_partial(datetime, zone)?);
        }
    }
    Ok(whens)
}

/// Build the interval a VEVENT or override names. An explicit end wins over a
/// duration, and a start with neither gets the RFC default length.
pub(crate) fn interval_from_entries(
    start_entry: &ICalendarEntry,
    end_entry: Option<&ICalendarEntry>,
    duration_entry: Option<&ICalendarEntry>,
    resolver: &TzResolver<&str>,
) -> Result<Interval, IcsError> {
    let start = when_from_entry(start_entry, resolver)?;

    if let Some(end_entry) = end_entry {
        let end = when_from_entry(end_entry, resolver)?;
        return Interval::new(start, end).map_err(temporal_error);
    }

    if let Some(duration) = duration_from_entry(duration_entry) {
        let end = add_duration_to_start(&start, duration)?;
        return Interval::new(start, end).map_err(temporal_error);
    }

    let end = match &start {
        When::Date { date } => When::Date {
            date: date.tomorrow().map_err(jiff_error)?,
        },
        When::Floating { datetime } => When::Floating {
            datetime: add_default_minutes(*datetime)?,
        },
        When::Instant { timestamp } => When::Instant {
            timestamp: add_default_minutes_to_instant(*timestamp)?,
        },
        When::Zoned(zoned) => When::Zoned(
            Zoned::new(add_default_minutes(zoned.datetime())?, zoned.zone())
                .map_err(temporal_error)?,
        ),
    };
    Interval::new(start, end).map_err(temporal_error)
}

/// Add a DURATION to a start while keeping the start's `When` kind, which is
/// what makes an all-day duration stay in whole days.
pub(crate) fn add_duration_to_start(
    start: &When,
    duration: &ICalendarDuration,
) -> Result<When, IcsError> {
    if duration.neg {
        return Err(IcsError::InvalidTemporal(
            "negative event duration".to_owned(),
        ));
    }

    let span = Span::new()
        .weeks(i64::from(duration.weeks))
        .days(i64::from(duration.days))
        .hours(i64::from(duration.hours))
        .minutes(i64::from(duration.minutes))
        .seconds(i64::from(duration.seconds));

    match start {
        When::Date { date } => {
            let days = i64::from(duration.weeks) * 7 + i64::from(duration.days);
            Ok(When::Date {
                date: date
                    .checked_add(Span::new().days(days))
                    .map_err(jiff_error)?,
            })
        }
        When::Floating { datetime } => Ok(When::Floating {
            datetime: datetime.checked_add(span).map_err(jiff_error)?,
        }),
        When::Instant { timestamp } => Ok(When::Instant {
            timestamp: timestamp.checked_add(span).map_err(jiff_error)?,
        }),
        When::Zoned(zoned) => Ok(When::Zoned(
            Zoned::new(
                zoned.datetime().checked_add(span).map_err(jiff_error)?,
                zoned.zone(),
            )
            .map_err(temporal_error)?,
        )),
    }
}

/// Build a date or time property in the shape the `When` calls for: a DATE
/// value, a floating local time, a UTC instant, or a zoned local time with its
/// TZID parameter.
pub(crate) fn entry_from_when(name: ICalendarProperty, when: &When) -> ICalendarEntry {
    ICalendarEntry {
        name,
        params: params_from_when(when),
        values: vec![ICalendarValue::PartialDateTime(Box::new(
            partial_from_when(when),
        ))],
    }
}

/// Build one property holding every value, using the series start's kind for
/// the shared parameter shape (VALUE=DATE or TZID).
pub(crate) fn multi_entry(
    name: ICalendarProperty,
    kind_source: &When,
    whens: &[When],
) -> ICalendarEntry {
    ICalendarEntry {
        name,
        params: params_from_when(kind_source),
        values: whens
            .iter()
            .map(|when| ICalendarValue::PartialDateTime(Box::new(partial_from_when(when))))
            .collect(),
    }
}

/// A DTSTAMP is always written as a UTC date-time, which calcard serializes
/// with a trailing `Z`.
pub(crate) fn partial_from_timestamp(timestamp: Timestamp) -> PartialDateTime {
    let datetime = timestamp.to_zoned_datetime(TimeZone::UTC).datetime();
    datetime_partial(datetime, Some(0), Some(0))
}

fn partial_from_when(when: &When) -> PartialDateTime {
    match when {
        When::Date { date } => PartialDateTime {
            year: Some(date.year() as u16),
            month: Some(date.month() as u8),
            day: Some(date.day() as u8),
            ..Default::default()
        },
        When::Floating { datetime } => datetime_partial(*datetime, None, None),
        When::Instant { timestamp } => {
            let datetime = timestamp.to_zoned_datetime(TimeZone::UTC).datetime();
            datetime_partial(datetime, Some(0), Some(0))
        }
        When::Zoned(zoned) => datetime_partial(zoned.datetime(), None, None),
    }
}

fn params_from_when(when: &When) -> Vec<ICalendarParameter> {
    match when {
        When::Date { .. } => vec![ICalendarParameter::value(ICalendarValueType::Date)],
        When::Zoned(zoned) => vec![ICalendarParameter::tzid(zoned.zone().to_owned())],
        _ => Vec::new(),
    }
}

fn datetime_partial(
    datetime: civil::DateTime,
    tz_hour: Option<u8>,
    tz_minute: Option<u8>,
) -> PartialDateTime {
    PartialDateTime {
        year: Some(datetime.year() as u16),
        month: Some(datetime.month() as u8),
        day: Some(datetime.day() as u8),
        hour: Some(datetime.hour() as u8),
        minute: Some(datetime.minute() as u8),
        second: Some(datetime.second() as u8),
        tz_hour,
        tz_minute,
        tz_minus: false,
    }
}

/// Resolve every TZID the entry carries before reading its value, so an
/// unknown zone fails instead of silently becoming a floating time.
fn resolved_zone(
    entry: &ICalendarEntry,
    resolver: &TzResolver<&str>,
) -> Result<Option<Tz>, IcsError> {
    let mut zone = None;
    for param in &entry.params {
        if param.name != ICalendarParameterName::Tzid {
            continue;
        }
        let Some(name) = param.value.as_text() else {
            continue;
        };
        let resolved = resolver
            .resolve(name)
            .ok_or_else(|| IcsError::UnknownTimezone(name.to_owned()))?;
        zone = Some(resolved);
    }
    Ok(zone)
}

fn when_from_partial(datetime: &PartialDateTime, zone: Option<Tz>) -> Result<When, IcsError> {
    if datetime.hour.is_none() {
        return Ok(When::Date {
            date: date_from_partial(datetime)?,
        });
    }

    let civil = datetime_from_partial(datetime)?;
    match zone {
        Some(Tz::Tz(time_zone)) => Ok(When::Zoned(
            Zoned::new(civil, time_zone.name()).map_err(temporal_error)?,
        )),
        Some(Tz::Fixed(offset)) => Ok(When::Instant {
            timestamp: instant_from_offset(civil, i64::from(offset.local_minus_utc()))?,
        }),
        Some(Tz::Floating) => Ok(When::Floating { datetime: civil }),
        None => match datetime.tz_hour {
            Some(hour) => {
                let seconds =
                    offset_seconds(hour, datetime.tz_minute.unwrap_or(0), datetime.tz_minus);
                Ok(When::Instant {
                    timestamp: instant_from_offset(civil, seconds)?,
                })
            }
            None => Ok(When::Floating { datetime: civil }),
        },
    }
}

fn date_from_partial(datetime: &PartialDateTime) -> Result<civil::Date, IcsError> {
    let year = datetime.year.ok_or_else(|| missing("year"))? as i16;
    let month = datetime.month.ok_or_else(|| missing("month"))? as i8;
    let day = datetime.day.ok_or_else(|| missing("day"))? as i8;
    civil::Date::new(year, month, day).map_err(jiff_error)
}

fn datetime_from_partial(datetime: &PartialDateTime) -> Result<civil::DateTime, IcsError> {
    let date = date_from_partial(datetime)?;
    civil::DateTime::new(
        date.year(),
        date.month(),
        date.day(),
        datetime.hour.unwrap_or(0) as i8,
        datetime.minute.unwrap_or(0) as i8,
        datetime.second.unwrap_or(0) as i8,
        0,
    )
    .map_err(jiff_error)
}

/// Turn a local civil time plus its UTC offset into the exact instant. The
/// offset is seconds east of UTC, so it is subtracted from the local time.
fn instant_from_offset(
    datetime: civil::DateTime,
    offset_seconds: i64,
) -> Result<Timestamp, IcsError> {
    let utc = datetime
        .to_zoned_datetime(TimeZone::UTC)
        .map_err(jiff_error)?
        .timestamp();
    utc.checked_sub(Span::new().seconds(offset_seconds))
        .map_err(jiff_error)
}

fn offset_seconds(hour: u8, minute: u8, minus: bool) -> i64 {
    let seconds = i64::from(hour) * 3600 + i64::from(minute) * 60;
    if minus { -seconds } else { seconds }
}

fn duration_from_entry(entry: Option<&ICalendarEntry>) -> Option<&ICalendarDuration> {
    match entry?.values.first()? {
        ICalendarValue::Duration(duration) => Some(duration),
        _ => None,
    }
}

fn add_default_minutes(datetime: civil::DateTime) -> Result<civil::DateTime, IcsError> {
    datetime
        .checked_add(Span::new().minutes(DEFAULT_EVENT_MINUTES))
        .map_err(jiff_error)
}

fn add_default_minutes_to_instant(timestamp: Timestamp) -> Result<Timestamp, IcsError> {
    timestamp
        .checked_add(Span::new().minutes(DEFAULT_EVENT_MINUTES))
        .map_err(jiff_error)
}

fn missing(part: &str) -> IcsError {
    IcsError::InvalidTemporal(format!("date is missing its {part}"))
}

fn jiff_error(error: jiff::Error) -> IcsError {
    IcsError::InvalidTemporal(error.to_string())
}

fn temporal_error(error: scry_core::temporal::TemporalError) -> IcsError {
    IcsError::InvalidTemporal(error.to_string())
}
