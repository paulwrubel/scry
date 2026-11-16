//! A temporal value is an all-day date, a floating local time, or an absolute instant.
//! The JSON form is also the storage form.
//! Zones are not stored, and DST resolution is deliberately absent until a feature needs it.

use jiff::Timestamp;
use jiff::civil;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum When {
    Date { date: civil::Date },
    Floating { datetime: civil::DateTime },
    Instant { timestamp: Timestamp },
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
    pub fn new(start: When, end: When) -> Result<Self, IntervalError> {
        let ordered = match (&start, &end) {
            (When::Date { date: start }, When::Date { date: end }) => end > start,
            (When::Floating { datetime: start }, When::Floating { datetime: end }) => end > start,
            (When::Instant { timestamp: start }, When::Instant { timestamp: end }) => end > start,
            _ => return Err(IntervalError::MismatchedKinds),
        };
        if !ordered {
            return Err(IntervalError::EndNotAfterStart);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalError {
    MismatchedKinds,
    EndNotAfterStart,
}

impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedKinds => f.write_str("interval start and end must be the same kind"),
            Self::EndNotAfterStart => f.write_str("interval end must be after its start"),
        }
    }
}

impl std::error::Error for IntervalError {}

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
            Err(IntervalError::MismatchedKinds)
        ));
        assert!(matches!(
            Interval::new(floating_when(9), instant()),
            Err(IntervalError::MismatchedKinds)
        ));
    }

    #[test]
    fn rejects_equal_and_reversed_endpoints() {
        assert!(matches!(
            Interval::new(date_when(3), date_when(3)),
            Err(IntervalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(date_when(4), date_when(3)),
            Err(IntervalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(floating_when(9), floating_when(9)),
            Err(IntervalError::EndNotAfterStart)
        ));
        assert!(matches!(
            Interval::new(floating_when(10), floating_when(9)),
            Err(IntervalError::EndNotAfterStart)
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
