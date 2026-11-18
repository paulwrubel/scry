//! Import a VCALENDAR into scry series. A master VEVENT becomes one series;
//! RECURRENCE-ID components become overrides, except `RANGE=THISANDFUTURE`,
//! which splits the series at that occurrence.

use crate::IcsEvent;
use crate::error::IcsError;
use crate::rrule::recurrence_from_rule;
use crate::temporal::{interval_from_entries, when_from_entry, whens_from_entry};
use calcard::icalendar::timezone::TzResolver;
use calcard::icalendar::{
    ICalendar, ICalendarComponent, ICalendarComponentType, ICalendarEntry, ICalendarParameterName,
    ICalendarParameterValue, ICalendarProperty, ICalendarValue,
};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use scry_core::event::split_series;
use scry_core::models::{
    Event, EventExternalMetadata, EventOverride, ExternalParameter, ExternalProperty,
};
use scry_core::recurrence::{RecurError, Recurrence, occurrences};
use scry_core::temporal::{Interval, When};

/// Validate every RRULE property in the source before handing the document to
/// calcard. calcard drops a rule it cannot parse, which would otherwise turn a
/// broken series into a single event with no error.
fn validate_rrule_lines(input: &str) -> Result<(), IcsError> {
    for line in logical_lines(input) {
        let Some((header, value)) = line.split_once(':') else {
            continue;
        };
        if !header
            .split(';')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("RRULE"))
        {
            continue;
        }

        if let Err(error) = Recurrence::parse(value) {
            let text = value.to_owned();
            return Err(match error {
                RecurError::Unsupported(_) => IcsError::UnsupportedRrule(text),
                _ => IcsError::InvalidRrule(text),
            });
        }
    }
    Ok(())
}

/// Unfold RFC 5545 continuation lines: a line starting with a space or tab
/// continues the one before it, and the fold whitespace is not part of the value.
fn logical_lines(input: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in input.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let continuation = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t'));
        match continuation {
            Some(rest) if !lines.is_empty() => {
                if let Some(last) = lines.last_mut() {
                    last.push_str(rest);
                }
            }
            _ => lines.push(line.to_owned()),
        }
    }
    lines
}

pub fn import(input: &str) -> Result<Vec<IcsEvent>, IcsError> {
    validate_rrule_lines(input)?;
    let ical = ICalendar::parse(input).map_err(|entry| IcsError::Parse(format!("{entry:?}")))?;
    let resolver = ical.build_tz_resolver();

    let vevents: Vec<&ICalendarComponent> = ical
        .components
        .iter()
        .filter(|component| component.component_type == ICalendarComponentType::VEvent)
        .collect();

    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, component) in vevents.iter().copied().enumerate() {
        let uid = text_property(component, &ICalendarProperty::Uid)
            .filter(|uid| !uid.is_empty())
            .unwrap_or_else(|| format!("{index}@scry-import"));
        match groups
            .iter_mut()
            .find(|(existing, _)| existing.as_str() == uid.as_str())
        {
            Some((_, members)) => members.push(index),
            None => groups.push((uid, vec![index])),
        }
    }

    let mut events = Vec::new();
    for (uid, members) in groups {
        events.extend(build_group(&uid, &members, &vevents, &resolver)?);
    }
    Ok(events)
}

fn build_group(
    uid: &str,
    members: &[usize],
    vevents: &[&ICalendarComponent],
    resolver: &TzResolver<&str>,
) -> Result<Vec<IcsEvent>, IcsError> {
    let master_index = members
        .iter()
        .copied()
        .find(|index| {
            vevents[*index]
                .property(&ICalendarProperty::RecurrenceId)
                .is_none()
        })
        .ok_or_else(|| IcsError::MissingMaster(uid.to_owned()))?;
    let master = vevents[master_index];

    let start_entry = master
        .property(&ICalendarProperty::Dtstart)
        .ok_or_else(|| IcsError::MissingStart(uid.to_owned()))?;
    let interval = interval_from_entries(
        start_entry,
        master.property(&ICalendarProperty::Dtend),
        master.property(&ICalendarProperty::Duration),
        resolver,
    )?;

    let recurrence = match master.property(&ICalendarProperty::Rrule) {
        Some(entry) => {
            let rule = match entry.values.first() {
                Some(ICalendarValue::RecurrenceRule(rule)) => rule,
                _ => {
                    return Err(IcsError::InvalidRrule(
                        entry
                            .values
                            .first()
                            .and_then(|value| value.clone().into_text())
                            .map(|text| text.into_owned())
                            .unwrap_or_default(),
                    ));
                }
            };
            Some(recurrence_from_rule(
                rule,
                interval.start(),
                collect_whens(master, &ICalendarProperty::Exdate, resolver)?,
                collect_whens(master, &ICalendarProperty::Rdate, resolver)?,
            )?)
        }
        None => None,
    };

    let external_metadata = EventExternalMetadata {
        uid: Some(uid.to_owned()),
        sequence: component_sequence(master),
        dtstamp: component_dtstamp(master, resolver)?,
        location: text_property(master, &ICalendarProperty::Location),
        status: text_property(master, &ICalendarProperty::Status),
        extra: unmapped_properties(master),
    };

    let mut series = IcsEvent {
        title: text_property(master, &ICalendarProperty::Summary)
            .unwrap_or_else(|| "(untitled)".to_owned()),
        description: text_property(master, &ICalendarProperty::Description),
        interval,
        recurrence,
        overrides: Vec::new(),
        external_metadata,
    };

    let mut overrides = Vec::new();
    for index in members
        .iter()
        .copied()
        .filter(|index| *index != master_index)
    {
        overrides.push(parsed_override(vevents[index], resolver)?);
    }
    overrides.sort_by_key(|overridden| instant_key(&overridden.event.original_start));

    let mut output = Vec::new();
    let mut suffix = 0u32;
    for overridden in overrides {
        if !overridden.this_and_future {
            series.overrides.push(overridden.event);
            continue;
        }

        let zone = series_zone(&series.interval)?;
        let probe = temporary_event(&series);
        match split_series(&probe, &overridden.event.original_start, &zone)
            .map_err(|error| IcsError::InvalidTemporal(error.to_string()))?
        {
            Some(split) => {
                suffix += 1;
                let interval = match &overridden.event.interval {
                    Some(interval) => interval.clone(),
                    None => generated_interval(&series, &overridden.event.original_start, &zone)?,
                };
                let title = overridden
                    .event
                    .title
                    .clone()
                    .unwrap_or_else(|| series.title.clone());
                let description = overridden
                    .event
                    .description
                    .clone()
                    .or_else(|| series.description.clone());
                let mut external_metadata = series.external_metadata.clone();
                external_metadata.uid = Some(format!("{uid}#{suffix}"));

                let mut finished = series;
                finished.recurrence = Some(split.truncated);
                finished.overrides = split.kept_overrides;
                output.push(finished);

                series = IcsEvent {
                    title,
                    description,
                    interval,
                    recurrence: Some(split.following),
                    overrides: split.following_overrides,
                    external_metadata,
                };
            }
            // the recurrence id is at or before the series start, so there is
            // nothing to truncate and the edit applies to that occurrence only
            None => series.overrides.push(overridden.event),
        }
    }
    output.push(series);

    Ok(output)
}

struct ParsedOverride {
    event: EventOverride,
    this_and_future: bool,
}

fn parsed_override(
    component: &ICalendarComponent,
    resolver: &TzResolver<&str>,
) -> Result<ParsedOverride, IcsError> {
    let recurrence_id = component
        .property(&ICalendarProperty::RecurrenceId)
        .ok_or_else(|| {
            IcsError::InvalidTemporal(
                "VEVENT has neither a master DTSTART nor a RECURRENCE-ID".to_owned(),
            )
        })?;

    let interval = match component.property(&ICalendarProperty::Dtstart) {
        Some(start) => Some(interval_from_entries(
            start,
            component.property(&ICalendarProperty::Dtend),
            component.property(&ICalendarProperty::Duration),
            resolver,
        )?),
        None => None,
    };

    Ok(ParsedOverride {
        event: EventOverride {
            original_start: when_from_entry(recurrence_id, resolver)?,
            title: text_property(component, &ICalendarProperty::Summary),
            description: text_property(component, &ICalendarProperty::Description),
            interval,
        },
        this_and_future: is_this_and_future(recurrence_id),
    })
}

fn is_this_and_future(entry: &ICalendarEntry) -> bool {
    match entry.parameter(&ICalendarParameterName::Range) {
        Some(ICalendarParameterValue::Bool(true)) => true,
        Some(ICalendarParameterValue::Text(value)) => value.eq_ignore_ascii_case("THISANDFUTURE"),
        _ => false,
    }
}

fn collect_whens(
    component: &ICalendarComponent,
    property: &ICalendarProperty,
    resolver: &TzResolver<&str>,
) -> Result<Vec<When>, IcsError> {
    let mut whens = Vec::new();
    for entry in component
        .entries
        .iter()
        .filter(|entry| &entry.name == property)
    {
        whens.extend(whens_from_entry(entry, resolver)?);
    }
    Ok(whens)
}

fn text_property(component: &ICalendarComponent, property: &ICalendarProperty) -> Option<String> {
    component
        .property(property)
        .and_then(|entry| entry.values.first())
        .and_then(|value| value.clone().into_text())
        .map(|text| text.into_owned())
}

fn component_sequence(component: &ICalendarComponent) -> u32 {
    component
        .property(&ICalendarProperty::Sequence)
        .and_then(|entry| entry.values.first())
        .and_then(|value| value.as_integer())
        .and_then(|sequence| u32::try_from(sequence).ok())
        .unwrap_or(0)
}

fn component_dtstamp(
    component: &ICalendarComponent,
    resolver: &TzResolver<&str>,
) -> Result<Option<Timestamp>, IcsError> {
    let Some(entry) = component.property(&ICalendarProperty::Dtstamp) else {
        return Ok(None);
    };
    let when = when_from_entry(entry, resolver)?;
    Ok(Some(match when {
        When::Instant { timestamp } => timestamp,
        other => other
            .start_instant(&TimeZone::UTC)
            .map_err(|error| IcsError::InvalidTemporal(error.to_string()))?,
    }))
}

fn unmapped_properties(component: &ICalendarComponent) -> Vec<ExternalProperty> {
    component
        .entries
        .iter()
        .filter(|entry| !is_mapped_property(&entry.name))
        .map(|entry| ExternalProperty {
            name: entry.name.as_str().to_owned(),
            params: entry
                .params
                .iter()
                .map(|param| ExternalParameter {
                    name: param.name.as_str().to_owned(),
                    value: param
                        .value
                        .clone()
                        .into_text()
                        .map(|text| text.into_owned())
                        .unwrap_or_default(),
                })
                .collect(),
            value: entry
                .values
                .first()
                .and_then(|value| value.clone().into_text())
                .map(|text| text.into_owned())
                .unwrap_or_default(),
        })
        .collect()
}

fn is_mapped_property(name: &ICalendarProperty) -> bool {
    matches!(
        name,
        ICalendarProperty::Begin
            | ICalendarProperty::End
            | ICalendarProperty::Uid
            | ICalendarProperty::Summary
            | ICalendarProperty::Description
            | ICalendarProperty::Dtstart
            | ICalendarProperty::Dtend
            | ICalendarProperty::Duration
            | ICalendarProperty::Rrule
            | ICalendarProperty::Exdate
            | ICalendarProperty::Rdate
            | ICalendarProperty::RecurrenceId
            | ICalendarProperty::Sequence
            | ICalendarProperty::Dtstamp
            | ICalendarProperty::Location
            | ICalendarProperty::Status
    )
}

fn instant_key(when: &When) -> Timestamp {
    when.start_instant(&TimeZone::UTC)
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

fn series_zone(interval: &Interval) -> Result<TimeZone, IcsError> {
    match interval.start() {
        When::Zoned(zoned) => TimeZone::get(zoned.zone())
            .map_err(|_| IcsError::UnknownTimezone(zoned.zone().to_owned())),
        _ => Ok(TimeZone::UTC),
    }
}

fn temporary_event(series: &IcsEvent) -> Event {
    Event {
        id: Default::default(),
        project_id: Default::default(),
        title: series.title.clone(),
        description: series.description.clone(),
        interval: series.interval.clone(),
        recurrence: series.recurrence.clone(),
        overrides: series.overrides.clone(),
        external_metadata: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    }
}

/// The occurrence the series generates at `at`, found by expanding a one-second
/// probe window so an EXDATE-removed start stays missing.
fn generated_interval(series: &IcsEvent, at: &When, zone: &TimeZone) -> Result<Interval, IcsError> {
    let Some(recurrence) = &series.recurrence else {
        return Ok(series.interval.clone());
    };
    let from = at
        .start_instant(zone)
        .map_err(|error| IcsError::InvalidTemporal(error.to_string()))?;
    let to = from
        .checked_add(Span::new().seconds(1))
        .map_err(|error| IcsError::InvalidTemporal(error.to_string()))?;
    let expanded = occurrences(&series.interval, recurrence, from, to, zone)
        .map_err(|error| IcsError::InvalidTemporal(error.to_string()))?;

    expanded
        .into_iter()
        .find(|interval| {
            interval
                .start_instant(zone)
                .is_ok_and(|start| start == from)
        })
        .ok_or_else(|| {
            IcsError::InvalidTemporal(
                "no occurrence starts at the THISANDFUTURE recurrence id".to_owned(),
            )
        })
}
