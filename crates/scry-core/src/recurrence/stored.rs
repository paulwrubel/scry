//! The stored recurrence value: the RRULE text plus the occurrence starts the
//! user added or removed. Validation lives here because both a write and a read
//! need the same kind check before an RDATE or EXDATE can be compared with a
//! series start.

use crate::temporal::When;
use serde::{Deserialize, Serialize};

use super::error::RecurError;
use super::recurrence_rule::Recurrence;

/// The recurrence stored on an event: the RRULE text, occurrence starts the user
/// added, and starts the user removed. Occurrences themselves are never stored.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RRule {
    /// The RRULE text, for example `FREQ=WEEKLY;BYDAY=MO`.
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exdates: Vec<When>,
    /// Additional starts added to the set, which the RRULE itself does not generate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rdates: Vec<When>,
}

impl RRule {
    /// Parse the stored text into `Recurrence` and reject an RDATE or EXDATE
    /// whose kind differs from the series start, since a mixed kind would need a
    /// caller zone to compare.
    pub fn validate(&self, start: &When) -> Result<Recurrence, RecurError> {
        let recurrence = Recurrence::parse(&self.text)?;
        if self
            .exdates
            .iter()
            .chain(&self.rdates)
            .any(|value| !same_kind(start, value))
        {
            return Err(RecurError::ExdateKindMismatch);
        }
        Ok(recurrence)
    }
}

/// Whether two values share a `When` kind, which is the only comparison an
/// RDATE or EXDATE needs because the occurrence itself supplies the zone.
fn same_kind(left: &When, right: &When) -> bool {
    matches!(
        (left, right),
        (When::Date { .. }, When::Date { .. })
            | (When::Floating { .. }, When::Floating { .. })
            | (When::Instant { .. }, When::Instant { .. })
            | (When::Zoned(_), When::Zoned(_))
    )
}
