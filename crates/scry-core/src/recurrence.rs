//! Hand-written RRULE expansion. The rule text is the source of truth; each
//! occurrence is generated as a civil datetime and rebuilt as the same `When`
//! kind, so a zoned series keeps its wall clock across a DST change. Every
//! boundary resolves through `When::start_instant`.
//!
//! The module is split by role: `stored` holds the stored value (`RRule`, the
//! text plus excluded starts), `recurrence_rule` parses and formats that text
//! into the typed `Recurrence`, `expand` generates occurrences, and `error` is the
//! failure shared by all three. Keeping the rule as text on the event means the
//! wire and snapshot formats carry a plain string, while `Recurrence` exists
//! only for the duration of a write or a window read.

mod error;
pub use error::RecurError;

mod expand;
pub use expand::occurrences;

mod recurrence_rule;
pub use recurrence_rule::{ByDay, Frequency, Recurrence, Until};

mod stored;
pub use stored::RRule;

#[cfg(test)]
mod tests;
