//! `event` commands.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::ids::Handle;
use scry_core::models::{Event, Project};
use scry_core::temporal::Interval;
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

    if output_json {
        print_json(&events)?;
        return Ok(());
    }

    if events.is_empty() {
        println!("No events in project \"{}\"", project.name);
        return Ok(());
    }

    let zone = viewer_zone()?;
    let now = Timestamp::now();
    for event in &events {
        println!("{}", event_line(event, &display_handle(event), now, &zone));
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
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
    let interval = interval_from(start, end, floating)?;

    #[derive(Serialize)]
    struct CreateEventRequest<'a> {
        title: &'a str,
        description: Option<&'a str>,
        interval: &'a Interval,
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
    output_json: bool,
) -> Result<(), AppError> {
    if title.is_none() && description.is_none() && start.is_none() {
        return Err(AppError::Usage(
            "Specify at least one of --title, --description, --start, or --end.".to_string(),
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

    #[derive(Serialize)]
    struct UpdateEventRequest<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        interval: Option<&'a Interval>,
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

/// Delete an event by handle.
pub async fn event_delete(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
    handle: &Handle,
    output_json: bool,
) -> Result<(), AppError> {
    let project: Project = project_by_name(client, config, project_flag).await?;
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

    println!(
        "When:        {}",
        scry_core::format::format_interval(&event.interval, Timestamp::now(), &viewer_zone()?)
    );
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
