//! `event` commands.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::ids::Handle;
use scry_core::models::{Event, Project};
use scry_core::recurrence::{RRule, Recurrence, occurrences};
use scry_core::temporal::{Interval, When};
use serde::Serialize;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::output::{event_line, print_json};
use crate::state::format_local;

use super::{parse_when_arg, project_by_name, system_zone};

/// List a project's events in start order.
pub async fn event_list(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let query = window_query(from, to)?;
    let pairs: Vec<(&str, &str)> = query
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();

    let project_id = project.id.to_string();
    let events: Vec<Event> = client
        .get_json(
            &["projects", project_id.as_str(), "events"],
            pairs.as_slice(),
        )
        .await?;

    let zone = viewer_zone()?;
    let window = match (from, to) {
        (Some(from), Some(to)) => Some((instant_of(from, &zone)?, instant_of(to, &zone)?)),
        _ => None,
    };

    let mut expanded: Vec<Event> = Vec::new();
    for event in events {
        let intervals = match (&event.recurrence, window) {
            (Some(rule), Some((start, end))) => Some(
                occurrences(&event.interval, rule, start, end, &zone)
                    .map_err(|error| AppError::Usage(error.to_string()))?,
            ),
            _ => None,
        };
        match intervals {
            Some(intervals) => {
                for interval in intervals {
                    let mut occurrence = event.clone();
                    occurrence.interval = interval;
                    expanded.push(occurrence);
                }
            }
            None => expanded.push(event),
        }
    }

    let mut rows = expanded
        .into_iter()
        .map(|event| {
            let start = event
                .interval
                .start()
                .start_instant(&zone)
                .map_err(|error| AppError::Usage(error.to_string()))?;
            Ok((start, event))
        })
        .collect::<Result<Vec<(Timestamp, Event)>, AppError>>()?;
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    let events: Vec<Event> = rows.into_iter().map(|(_, event)| event).collect();

    if output_json {
        print_json(&events)?;
        return Ok(());
    }

    if events.is_empty() {
        println!("No events in project \"{}\"", project.name);
        return Ok(());
    }

    let now = Timestamp::now();
    for event in &events {
        let mut line = event_line(event, &event.interval, &display_handle(event), now, &zone);
        if event.recurrence.is_some() {
            line.push_str(" [repeats]");
        }
        println!("{line}");
    }

    Ok(())
}

/// Add an event to a project.
#[allow(clippy::too_many_arguments)]
pub async fn event_add(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    title: &str,
    description: Option<&str>,
    start: &str,
    end: Option<&str>,
    floating: bool,
    recur: Option<&str>,
    until: Option<&str>,
    count: Option<u32>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let interval = interval_from(start, end, floating)?;
    let recurrence = recur
        .map(|text| build_rrule(text, until, count, floating))
        .transpose()?;

    #[derive(Serialize)]
    struct CreateEventRequest<'a> {
        title: &'a str,
        description: Option<&'a str>,
        interval: &'a Interval,
        #[serde(skip_serializing_if = "Option::is_none")]
        recurrence: Option<&'a RRule>,
    }

    let project_id = project.id.to_string();
    let event: Event = client
        .post_json(
            &["projects", project_id.as_str(), "events"],
            &[],
            &CreateEventRequest {
                title,
                description,
                interval: &interval,
                recurrence: recurrence.as_ref(),
            },
        )
        .await?;

    if output_json {
        print_json(&event)?;
    } else {
        println!(
            "Added event \"{}\" to project \"{}\"",
            event.title, project.name
        );
    }

    Ok(())
}

/// Update an event's title, description, or interval.
#[allow(clippy::too_many_arguments)]
pub async fn event_update(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    handle: &Handle,
    title: Option<&str>,
    description: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    floating: bool,
    recur: Option<&str>,
    until: Option<&str>,
    count: Option<u32>,
    no_recur: bool,
    output_json: bool,
) -> Result<(), AppError> {
    if title.is_none() && description.is_none() && start.is_none() && recur.is_none() && !no_recur {
        return Err(AppError::Usage(
            "Specify at least one of --title, --description, --start, --end, or a recurrence flag."
                .to_string(),
        ));
    }

    let project: Project = project_by_name(client, config, project_flag).await?;
    let event = event_by_handle(client, project.id, handle).await?;
    let interval = match (start, end) {
        (Some(start), Some(end)) => Some(interval_from(start, Some(end), floating)?),
        (None, None) => None,
        _ => {
            return Err(AppError::Usage(
                "--start and --end must be used together.".to_string(),
            ));
        }
    };

    let recurrence: Option<Option<RRule>> = if no_recur {
        Some(None)
    } else if let Some(text) = recur {
        let mut rule = build_rrule(text, until, count, floating)?;
        if let Some(existing) = &event.recurrence {
            rule.exdates = existing.exdates.clone();
        }
        Some(Some(rule))
    } else {
        None
    };

    #[derive(Serialize)]
    struct UpdateEventRequest<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        interval: Option<&'a Interval>,
        #[serde(skip_serializing_if = "Option::is_none")]
        recurrence: Option<Option<&'a RRule>>,
    }

    let event_id = event.id.to_string();
    let updated: Event = client
        .put_json(
            &["events", event_id.as_str()],
            &[],
            &UpdateEventRequest {
                title,
                description,
                interval: interval.as_ref(),
                recurrence: recurrence.as_ref().map(Option::as_ref),
            },
        )
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        println!("Updated event \"{}\"", updated.title);
    }

    Ok(())
}

/// Delete an event by handle, or one occurrence of a recurring event.
pub async fn event_delete(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    handle: &Handle,
    occurrence: Option<&str>,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;

    if let Some(expression) = occurrence {
        return delete_occurrence(client, project, handle, expression, output_json).await;
    }

    let event = event_by_handle(client, project.id, handle).await?;
    let event_id = event.id.to_string();

    let deleted: Event = client
        .delete_json(&["events", event_id.as_str()], &[])
        .await?;

    if output_json {
        print_json(&deleted)?;
    } else {
        println!("Deleted event \"{}\"", deleted.title);
    }

    Ok(())
}

/// Exclude one occurrence of a recurring event by adding an EXDATE.
async fn delete_occurrence(
    client: &Client,
    project: Project,
    handle: &Handle,
    expression: &str,
    output_json: bool,
) -> Result<(), AppError> {
    let event = event_by_handle(client, project.id, handle).await?;
    let Some(existing) = event.recurrence.clone() else {
        return Err(AppError::Usage("Event is not recurring.".to_string()));
    };

    let zone = viewer_zone()?;
    let target = instant_of(expression, &zone)?;
    // a one-second window catches only the occurrence that starts at `target`
    let end = target
        .checked_add(jiff::Span::new().seconds(1))
        .map_err(|error| AppError::Usage(error.to_string()))?;
    let candidates = occurrences(&event.interval, &existing, target, end, &zone)
        .map_err(|error| AppError::Usage(error.to_string()))?;
    let Some(occurrence) = candidates.into_iter().find(|interval| {
        interval
            .start()
            .start_instant(&zone)
            .is_ok_and(|start| start == target)
    }) else {
        return Err(AppError::Usage(format!(
            "No occurrence starts at \"{expression}\""
        )));
    };

    let exdate = occurrence.start().clone();
    let mut rule = existing;
    if !rule.exdates.contains(&exdate) {
        rule.exdates.push(exdate);
    }

    #[derive(Serialize)]
    struct UpdateRecurrenceRequest<'a> {
        recurrence: &'a RRule,
    }

    let event_id = event.id.to_string();
    let updated: Event = client
        .put_json(
            &["events", event_id.as_str()],
            &[],
            &UpdateRecurrenceRequest { recurrence: &rule },
        )
        .await?;

    if output_json {
        print_json(&updated)?;
    } else {
        println!(
            "Deleted occurrence on {}",
            scry_core::format::format_interval(&occurrence, Timestamp::now(), &zone)
        );
    }

    Ok(())
}

/// Show one event.
pub async fn event_show(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    handle: &Handle,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let event = event_by_handle(client, project.id, handle).await?;

    if output_json {
        print_json(&event)?;
        return Ok(());
    }

    println!("{} #{}", event.title, display_handle(&event));
    println!();

    if let Some(description) = &event.description {
        for line in description.lines() {
            println!("    {}", line);
        }
        println!();
    }

    let now = Timestamp::now();
    let zone = viewer_zone()?;
    println!(
        "When:        {}",
        scry_core::format::format_interval(&event.interval, now, &zone)
    );
    if let Some(rule) = &event.recurrence {
        println!("Repeat:      {}", rule.text);
        if !rule.exdates.is_empty() {
            let exdates = rule
                .exdates
                .iter()
                .map(|exdate| scry_core::format::format_when(exdate, now, &zone))
                .collect::<Vec<_>>()
                .join(", ");
            println!("Excluded:    {exdates}");
        }
    }
    println!("Created at:  {}", format_local(event.created_at));
    println!("Updated at:  {}", format_local(event.updated_at));

    Ok(())
}

fn interval_from(start: &str, end: Option<&str>, floating: bool) -> Result<Interval, AppError> {
    let start = parse_when_arg(start, floating)?;
    match end {
        Some(end) => Interval::new(start, parse_when_arg(end, floating)?)
            .map_err(|error| AppError::Usage(error.to_string())),
        None => Interval::from_start(start).map_err(|error| AppError::Usage(error.to_string())),
    }
}

/// Build a stored rule from the CLI's rule text and optional bounds.
fn build_rrule(
    text: &str,
    until: Option<&str>,
    count: Option<u32>,
    floating: bool,
) -> Result<RRule, AppError> {
    let mut parts = vec![text.trim().to_string()];
    if let Some(count) = count {
        parts.push(format!("COUNT={count}"));
    }
    if let Some(until) = until {
        parts.push(format!("UNTIL={}", rrule_until(until, floating)?));
    }
    let text = parts.join(";");
    Recurrence::parse(&text).map_err(|error| AppError::Usage(error.to_string()))?;
    Ok(RRule {
        text,
        exdates: Vec::new(),
    })
}

/// Render a CLI end expression as an RRULE UNTIL value.
fn rrule_until(expr: &str, floating: bool) -> Result<String, AppError> {
    let when = parse_when_arg(expr, floating)?;
    match when {
        When::Date { date } => Ok(date.strftime("%Y%m%d").to_string()),
        other => {
            let instant = other
                .start_instant(&viewer_zone()?)
                .map_err(|error| AppError::Usage(error.to_string()))?;
            Ok(instant
                .to_zoned_datetime(TimeZone::UTC)
                .datetime()
                .strftime("%Y%m%dT%H%M%SZ")
                .to_string())
        }
    }
}

/// The `from`/`to` query pair, resolved in the system zone. Both bounds are
/// optional, but a window needs both.
fn window_query(from: Option<&str>, to: Option<&str>) -> Result<Vec<(String, String)>, AppError> {
    match (from, to) {
        (None, None) => Ok(Vec::new()),
        (Some(from), Some(to)) => {
            let zone = viewer_zone()?;
            let from = instant_of(from, &zone)?;
            let to = instant_of(to, &zone)?;
            Ok(vec![
                ("from".to_string(), from.to_string()),
                ("to".to_string(), to.to_string()),
            ])
        }
        _ => Err(AppError::Usage(
            "--from and --to must be used together.".to_string(),
        )),
    }
}

fn instant_of(expr: &str, zone: &TimeZone) -> Result<Timestamp, AppError> {
    parse_when_arg(expr, false)?
        .start_instant(zone)
        .map_err(|error| AppError::Usage(error.to_string()))
}

fn viewer_zone() -> Result<TimeZone, AppError> {
    let name = system_zone()?;
    TimeZone::get(&name).map_err(|_| AppError::Usage(format!("invalid system time zone: {name:?}")))
}

fn display_handle(event: &Event) -> String {
    let id = event.id.simple().to_string();
    id[..8].to_string()
}

/// Resolve a handle prefix against a project's events.
async fn event_by_handle(
    client: &Client,
    project_id: scry_core::ids::ProjectId,
    handle: &Handle,
) -> Result<Event, AppError> {
    let project_id = project_id.to_string();
    let events: Vec<Event> = client
        .get_json(&["projects", project_id.as_str(), "events"], &[])
        .await?;

    let prefix = handle.as_str();
    let matches: Vec<Event> = events
        .into_iter()
        .filter(|event| event.id.simple().to_string().starts_with(prefix))
        .collect();

    match matches.as_slice() {
        [] => Err(AppError::Usage(format!(
            "No event matches handle \"{prefix}\""
        ))),
        [event] => Ok(event.clone()),
        _ => Err(AppError::Usage(format!(
            "Handle \"{prefix}\" matches multiple events. Use a longer prefix."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_is_appended_to_the_rule() {
        let rule = build_rrule("FREQ=WEEKLY;BYDAY=MO", None, Some(5), false).unwrap();
        assert_eq!(rule.text, "FREQ=WEEKLY;BYDAY=MO;COUNT=5");
    }

    #[test]
    fn a_date_until_is_rendered_compactly() {
        let rule = build_rrule("FREQ=DAILY", Some("2026-12-31"), None, false).unwrap();
        assert_eq!(rule.text, "FREQ=DAILY;UNTIL=20261231");
    }

    #[test]
    fn an_invalid_rule_is_rejected() {
        assert!(build_rrule("FREQ=NOPE", None, None, false).is_err());
    }
}
