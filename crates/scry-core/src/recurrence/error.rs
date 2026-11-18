//! Failures raised while parsing an RRULE or expanding a series. One error type
//! is shared so the parser and the expander can compose without depending on
//! each other.

use std::fmt;

pub(super) const MAX_CANDIDATES: usize = 100_000;

/// Why a rule could not be parsed or expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurError {
    /// A token in the rule text was malformed.
    Parse { token: String, detail: String },
    /// The rule named a part this subset does not implement.
    Unsupported(String),
    /// An RDATE or EXDATE carried a different `When` kind from the series start.
    ExdateKindMismatch,
    /// Generation hit `MAX_CANDIDATES`, which guards against an endless series.
    OccurrenceLimit,
    /// A temporal value failed to resolve or build an interval.
    Invalid(String),
}

impl fmt::Display for RecurError {
    /// Render the failure for a CLI or API error message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { token, detail } => {
                write!(f, "invalid RRULE token {token:?}: {detail}")
            }
            Self::Unsupported(part) => write!(f, "unsupported RRULE part: {part}"),
            Self::ExdateKindMismatch => {
                f.write_str("excluded or included date must be the same kind as the series start")
            }
            Self::OccurrenceLimit => {
                write!(
                    f,
                    "recurrence exceeded {MAX_CANDIDATES} candidate occurrences"
                )
            }
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RecurError {}

/// Build a parse error that names the offending token.
pub(super) fn invalid(token: &str, detail: impl Into<String>) -> RecurError {
    RecurError::Parse {
        token: token.to_owned(),
        detail: detail.into(),
    }
}
