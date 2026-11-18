//! iCalendar (RFC 5545) import and export for scry events.
//!
//! Import turns every VEVENT group into one or more [`IcsEvent`] series: a
//! master plus its per-occurrence overrides, with a `RANGE=THISANDFUTURE`
//! override splitting the series at the occurrence it edits. Export writes the
//! same shape back, preserving the properties scry does not model through
//! `EventExternalMetadata::extra`.

mod error;
pub use error::IcsError;

mod export;
pub use export::{export, export_events};

mod import;
pub use import::import;

mod rrule;
mod temporal;

use scry_core::models::{Event, EventExternalMetadata, EventOverride};
use scry_core::recurrence::RRule;
use scry_core::temporal::Interval;

/// One calendar series as import and export see it: the fields scry models
/// plus the metadata needed to reproduce the source component.
#[derive(Debug, Clone, PartialEq)]
pub struct IcsEvent {
    pub title: String,
    pub description: Option<String>,
    pub interval: Interval,
    pub recurrence: Option<RRule>,
    pub overrides: Vec<EventOverride>,
    pub external_metadata: EventExternalMetadata,
}

/// Build an exportable series from a stored event, filling in the UID and
/// DTSTAMP an export needs when the event has no source metadata.
pub fn from_event(event: &Event) -> IcsEvent {
    let mut external_metadata = event.external_metadata.clone().unwrap_or_default();
    if external_metadata.uid.is_none() {
        let id = event.id;
        external_metadata.uid = Some(format!("{id}@scry"));
    }
    if external_metadata.dtstamp.is_none() {
        external_metadata.dtstamp = Some(event.updated_at);
    }

    IcsEvent {
        title: event.title.clone(),
        description: event.description.clone(),
        interval: event.interval.clone(),
        recurrence: event.recurrence.clone(),
        overrides: event.overrides.clone(),
        external_metadata,
    }
}
