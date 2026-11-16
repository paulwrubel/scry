//! A temporal value is an all-day date, a floating local time, an absolute
//! instant, or a local time in a named zone. The JSON form is also the storage
//! form. A zoned value resolves to an instant only in [`Zoned::to_instant`],
//! where jiff shifts a DST gap forward and picks the earlier instant of a fold.

use jiff::Timestamp;
use jiff::civil;
use jiff::tz::TimeZone;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum When {
    Date { date: civil::Date },
    Floating { datetime: civil::DateTime },
    Instant { timestamp: Timestamp },
    Zoned(Zoned),
}

/// A wall-clock time in a named IANA zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Zoned {
    datetime: civil::DateTime,
    zone: String,
}

impl Zoned {
    /// Store `datetime` under the canonical name of `zone`.
    pub fn new(datetime: civil::DateTime, zone: &str) -> Result<Self, TemporalError> {
        let name = TimeZone::get(zone)
            .ok()
            .and_then(|time_zone| time_zone.iana_name().map(str::to_owned))
            .ok_or_else(|| TemporalError::InvalidZone {
                name: zone.to_owned(),
            })?;
        Ok(Self {
            datetime,
            zone: name,
        })
    }

    /// The local wall-clock time, as written in the source (for example `.ics`).
    pub fn datetime(&self) -> civil::DateTime {
        self.datetime
    }

    /// The IANA zone name, as written in the source (for example `.ics`).
    pub fn zone(&self) -> &str {
        &self.zone
    }

    /// Resolve to an instant. jiff shifts a DST gap forward and picks the
    /// earlier instant of a fold; this is the only place that policy applies.
    pub fn to_instant(&self) -> Result<Timestamp, TemporalError> {
        let time_zone = TimeZone::get(&self.zone).map_err(|_| TemporalError::InvalidZone {
            name: self.zone.clone(),
        })?;
        Ok(time_zone
            .to_zoned_datetime(self.datetime)
            .map_err(TemporalError::jiff)?
            .timestamp())
    }
}

// Go through `Zoned::new` so an offset-only or unknown zone cannot enter.
impl<'de> Deserialize<'de> for Zoned {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            datetime: civil::DateTime,
            zone: String,
        }

        let raw = Raw::deserialize(deserializer)?;
        Self::new(raw.datetime, &raw.zone).map_err(serde::de::Error::custom)
    }
}

/// `start` is inclusive and `end` is exclusive, so adjacent ranges never overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Interval {
    start: When,
    end: When,
}

// Go through `Interval::new` so mixed kinds and a non-increasing end are
// rejected at the boundary; mixing kinds would need a zone to compare.
impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            start: When,
            end: When,
        }

        let raw = Raw::deserialize(deserializer)?;
        Self::new(raw.start, raw.end).map_err(serde::de::Error::custom)
    }
}

impl Interval {
    pub fn new(start: When, end: When) -> Result<Self, TemporalError> {
        let ordered = match (&start, &end) {
            (When::Date { date: start }, When::Date { date: end }) => end > start,
            (When::Floating { datetime: start }, When::Floating { datetime: end }) => end > start,
            (When::Instant { timestamp: start }, When::Instant { timestamp: end }) => end > start,
            (When::Zoned(start), When::Zoned(end)) => end.to_instant()? > start.to_instant()?,
            _ => return Err(TemporalError::MismatchedKinds),
        };
        if !ordered {
            return Err(TemporalError::EndNotAfterStart);
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> &When {
        &self.start
    }

    pub fn end(&self) -> &When {
        &self.end
    }

    /// Storage keeps an exclusive end while the UI shows the last covered day.
    pub fn inclusive_end_date(&self) -> Option<civil::Date> {
        match &self.end {
            When::Date { date: end } => end.yesterday().ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalError {
    MismatchedKinds,
    EndNotAfterStart,
    InvalidZone { name: String },
    Arithmetic(String),
}

impl TemporalError {
    fn jiff(error: jiff::Error) -> Self {
        Self::Arithmetic(error.to_string())
    }
}

impl fmt::Display for TemporalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedKinds => f.write_str("interval start and end must be the same kind"),
            Self::EndNotAfterStart => f.write_str("interval end must be after its start"),
            Self::InvalidZone { name } => write!(f, "invalid IANA time zone: {name:?}"),
            Self::Arithmetic(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for TemporalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    use serde_json::json;

    fn date_when(day: i8) -> When {
        When::Date {
            date: date(2026, 11, day),
        }
    }

    fn floating_when(hour: i8) -> When {
        When::Floating {
            datetime: date(2026, 11, 3).at(hour, 0, 0, 0),
        }
    }

    fn instant() -> When {
        When::Instant {
            timestamp: "2026-11-03T15:00:00Z".parse().unwrap(),
        }
    }

    fn zoned_when(day: i8, hour: i8, zone: &str) -> When {
        When::Zoned(Zoned::new(date(2026, 11, day).at(hour, 0, 0, 0), zone).unwrap())
    }

    #[test]
    fn when_json_round_trips() {
        let cases = [
            (date_when(3), json!({"kind": "date", "date": "2026-11-03"})),
            (
                floating_when(9),
                json!({"kind": "floating", "datetime": "2026-11-03T09:00:00"}),
            ),
            (
                instant(),
                json!({"kind": "instant", "timestamp": "2026-11-03T15:00:00Z"}),
            ),
            (
                zoned_when(3, 9, "America/Chicago"),
                json!({
                    "kind": "zoned",
                    "datetime": "2026-11-03T09:00:00",
                    "zone": "America/Chicago",
                }),
            ),
        ];

        for (when, value) in cases {
            assert_eq!(serde_json::to_value(&when).unwrap(), value);
            assert_eq!(serde_json::from_value::<When>(value).unwrap(), when);
        }
    }

    #[test]
    fn interval_json_round_trips() {
        let interval = Interval::new(date_when(3), date_when(6)).unwrap();

        let value = serde_json::to_value(&interval).unwrap();
        assert_eq!(
            value,
            json!({
                "start": {"kind": "date", "date": "2026-11-03"},
                "end": {"kind": "date", "date": "2026-11-06"},
            })
        );
        assert_eq!(serde_json::from_value::<Interval>(value).unwrap(), interval);
    }

    #[test]
    fn rejects_mismatched_interval_kinds() {
        assert!(matches!(
            Interval::new(date_when(3), instant()),
            Err(TemporalError::MismatchedKinds)
        ));
        assert!(matches!(
            Interval::new(floating_when(9), instant()),
            Err(TemporalError::MismatchedKinds)
        ));
        assert!(matches!(
            Interval::new(zoned_when(3, 9, "America/Chicago"), instant()),
            Err(TemporalError::MismatchedKinds)
        ));
    }

    #[test]
    fn rejects_offset_only_and_unknown_zones() {
        for zone in ["-06:00", "Mars/Olympus"] {
            let value = json!({
                "kind": "zoned",
                "datetime": "2026-11-03T09:00:00",
                "zone": zone,
            });
            assert!(
                serde_json::from_value::<When>(value).is_err(),
                "zone {zone} should be rejected"
            );
        }
    }

    #[test]
    fn zoned_resolves_gap_forward_and_fold_earlier() {
        let gap = Zoned::new(date(2026, 3, 8).at(2, 30, 0, 0), "America/Chicago").unwrap();
        assert_eq!(
            gap.to_instant().unwrap().to_string(),
            "2026-03-08T08:30:00Z"
        );

        let fold = Zoned::new(date(2026, 11, 1).at(1, 30, 0, 0), "America/Chicago").unwrap();
        assert_eq!(
            fold.to_instant().unwrap().to_string(),
            "2026-11-01T06:30:00Z"
        );
    }

    #[test]
    fn zoned_interval_compares_instants_across_zones() {
        let start = zoned_when(3, 9, "America/Chicago");
        let end = zoned_when(3, 11, "America/New_York");
        assert!(Interval::new(start, end).is_ok());

        let start = zoned_when(3, 11, "America/New_York");
        let end = zoned_when(3, 9, "America/Chicago");
        assert!(matches!(
            Interval::new(start, end),
            Err(TemporalError::EndNotAfterStart)
        ));
    }

    #[test]
    fn rejects_equal_and_reversed_endpoints() {
        assert!(matches!(
            Interval::new(date_when(3), date_when(3)),
            Err(TemporalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(date_when(4), date_when(3)),
            Err(TemporalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(floating_when(9), floating_when(9)),
            Err(TemporalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(floating_when(10), floating_when(9)),
            Err(TemporalError::EndNotAfterStart)
        ));
    }

    #[test]
    fn rejects_equal_endpoints_on_deserialize() {
        let value = json!({
            "start": {"kind": "date", "date": "2026-11-03"},
            "end": {"kind": "date", "date": "2026-11-03"},
        });
        assert!(serde_json::from_value::<Interval>(value).is_err());
    }

    #[test]
    fn inclusive_end_date_steps_back_one_day() {
        let all_day = Interval::new(date_when(3), date_when(6)).unwrap();
        assert_eq!(all_day.inclusive_end_date(), Some(date(2026, 11, 5)));

        let floating = Interval::new(floating_when(9), floating_when(10)).unwrap();
        assert_eq!(floating.inclusive_end_date(), None);
    }
}
