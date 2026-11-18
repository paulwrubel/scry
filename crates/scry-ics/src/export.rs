//! Export scry series as a VCALENDAR. The series fields map to the RFC 5545
//! properties scry understands, and the remaining properties are written back
//! from `EventExternalMetadata::extra` unchanged.

use crate::IcsEvent;
use crate::error::IcsError;
use crate::rrule::rule_from_recurrence;
use crate::temporal::{entry_from_when, multi_entry, partial_from_timestamp};
use calcard::icalendar::{
    ICalendar, ICalendarComponent, ICalendarComponentType, ICalendarEntry, ICalendarParameter,
    ICalendarParameterName, ICalendarParameterValue, ICalendarProperty, ICalendarValue,
};
use jiff::Timestamp;
use scry_core::models::{Event, EventOverride, ExternalProperty};

pub fn export(events: &[IcsEvent]) -> Result<String, IcsError> {
    let mut ical = ICalendar {
        components: vec![ICalendarComponent::new(ICalendarComponentType::VCalendar)],
    };
    ical.components[0].add_property(
        ICalendarProperty::Version,
        ICalendarValue::Text("2.0".to_owned()),
    );
    ical.components[0].add_property(
        ICalendarProperty::Prodid,
        ICalendarValue::Text("-//scry//scry-ics//EN".to_owned()),
    );

    for (index, event) in events.iter().enumerate() {
        for component in series_components(event, index)? {
            let id = ical.components.len() as u32;
            ical.components.push(component);
            ical.components[0].component_ids.push(id);
        }
    }

    ical.add_missing_timezones();
    Ok(ical.to_string())
}

pub fn export_events(events: &[Event]) -> Result<String, IcsError> {
    let series: Vec<IcsEvent> = events.iter().map(crate::from_event).collect();
    export(&series)
}

fn series_components(event: &IcsEvent, index: usize) -> Result<Vec<ICalendarComponent>, IcsError> {
    let metadata = &event.external_metadata;
    let uid = metadata
        .uid
        .clone()
        .filter(|uid| !uid.is_empty())
        .ok_or_else(|| IcsError::InvalidTemporal(format!("event at index {index} has no UID")))?;
    let dtstamp = metadata.dtstamp.unwrap_or_else(Timestamp::now);
    let sequence = i64::from(metadata.sequence);

    // the master is the VEVENT without a RECURRENCE-ID; overrides below carry one
    let mut master = ICalendarComponent::new(ICalendarComponentType::VEvent);
    master.add_uid(&uid);
    master.add_dtstamp(partial_from_timestamp(dtstamp));
    master.add_sequence(sequence);
    master.add_property(
        ICalendarProperty::Summary,
        ICalendarValue::Text(event.title.clone()),
    );
    if let Some(description) = &event.description {
        master.add_property(
            ICalendarProperty::Description,
            ICalendarValue::Text(description.clone()),
        );
    }
    if let Some(location) = &metadata.location {
        master.add_property(
            ICalendarProperty::Location,
            ICalendarValue::Text(location.clone()),
        );
    }
    if let Some(status) = &metadata.status {
        master.add_property(
            ICalendarProperty::Status,
            ICalendarValue::Text(status.clone()),
        );
    }
    master.entries.push(entry_from_when(
        ICalendarProperty::Dtstart,
        event.interval.start(),
    ));
    master.entries.push(entry_from_when(
        ICalendarProperty::Dtend,
        event.interval.end(),
    ));

    if let Some(recurrence) = &event.recurrence {
        let rule = rule_from_recurrence(recurrence, event.interval.start())?;
        master.add_property(
            ICalendarProperty::Rrule,
            ICalendarValue::RecurrenceRule(Box::new(rule)),
        );
        if !recurrence.exdates.is_empty() {
            master.entries.push(multi_entry(
                ICalendarProperty::Exdate,
                event.interval.start(),
                &recurrence.exdates,
            ));
        }
        if !recurrence.rdates.is_empty() {
            master.entries.push(multi_entry(
                ICalendarProperty::Rdate,
                event.interval.start(),
                &recurrence.rdates,
            ));
        }
    }

    for property in &metadata.extra {
        master.entries.push(extra_entry(property));
    }

    let mut components = vec![master];
    for overridden in &event.overrides {
        components.push(override_component(&uid, dtstamp, sequence, overridden));
    }
    Ok(components)
}

fn override_component(
    uid: &str,
    dtstamp: Timestamp,
    sequence: i64,
    overridden: &EventOverride,
) -> ICalendarComponent {
    let mut component = ICalendarComponent::new(ICalendarComponentType::VEvent);
    component.add_uid(uid);
    component.entries.push(entry_from_when(
        ICalendarProperty::RecurrenceId,
        &overridden.original_start,
    ));
    component.add_dtstamp(partial_from_timestamp(dtstamp));
    component.add_sequence(sequence);

    if let Some(title) = &overridden.title {
        component.add_property(
            ICalendarProperty::Summary,
            ICalendarValue::Text(title.clone()),
        );
    }
    if let Some(description) = &overridden.description {
        component.add_property(
            ICalendarProperty::Description,
            ICalendarValue::Text(description.clone()),
        );
    }
    if let Some(interval) = &overridden.interval {
        component.entries.push(entry_from_when(
            ICalendarProperty::Dtstart,
            interval.start(),
        ));
        component
            .entries
            .push(entry_from_when(ICalendarProperty::Dtend, interval.end()));
    }

    component
}

fn extra_entry(property: &ExternalProperty) -> ICalendarEntry {
    let mut entry = ICalendarEntry::new(ICalendarProperty::Other(property.name.clone()));
    entry.params = property
        .params
        .iter()
        .map(|param| {
            ICalendarParameter::new(
                ICalendarParameterName::parse(&param.name),
                ICalendarParameterValue::Text(param.value.clone()),
            )
        })
        .collect();
    entry
        .values
        .push(ICalendarValue::Text(property.value.clone()));
    entry
}
