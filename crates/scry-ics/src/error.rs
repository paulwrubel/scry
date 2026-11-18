//! The single error type shared by import, export, and rule validation. Every
//! variant names the input that failed so a caller can show it verbatim.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcsError {
    /// The input was not an iCalendar object calcard could parse.
    Parse(String),
    /// A UID had only overrides and no master VEVENT.
    MissingMaster(String),
    /// A master VEVENT had no DTSTART.
    MissingStart(String),
    /// A TZID did not resolve to an IANA or supported Windows zone.
    UnknownTimezone(String),
    /// calcard dropped the rule because its own grammar rejected it.
    InvalidRrule(String),
    /// calcard accepted the rule but scry's expander cannot represent it.
    UnsupportedRrule(String),
    /// A date, interval, or duration could not be built.
    InvalidTemporal(String),
}

impl fmt::Display for IcsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(formatter, "could not parse iCalendar: {message}"),
            Self::MissingMaster(uid) => {
                write!(formatter, "no master VEVENT for UID {uid:?}")
            }
            Self::MissingStart(uid) => {
                write!(formatter, "VEVENT with UID {uid:?} has no DTSTART")
            }
            Self::UnknownTimezone(name) => write!(
                formatter,
                "cannot resolve TZID {name:?}; it is not a known IANA or supported Windows time zone"
            ),
            Self::InvalidRrule(text) => write!(formatter, "invalid RRULE {text:?}"),
            Self::UnsupportedRrule(text) => write!(
                formatter,
                "scry's recurrence expander cannot represent RRULE {text:?}"
            ),
            Self::InvalidTemporal(message) => {
                write!(formatter, "invalid temporal value: {message}")
            }
        }
    }
}

impl std::error::Error for IcsError {}
