//! Shared human and JSON output helpers.

use crate::error::AppError;
use crate::state::TaskWithComments;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::format::{format_interval, format_when};
use scry_core::ids::TaskId;
use scry_core::models::{Event, Section, TaskState};
use scry_core::temporal::Interval;
use std::collections::HashMap;

/// Print a value as compact JSON on stdout.
pub fn print_json<T: serde::Serialize + ?Sized>(value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string(value)
        .map_err(|error| AppError::Internal(format!("failed to serialize JSON: {error}")))?;
    println!("{json}");
    Ok(())
}

/// The display label for a task's section.
pub fn section_label(section: &Option<Section>) -> &str {
    section
        .as_ref()
        .map_or("No section", |section| section.name.as_str())
}

/// Render one event line for the human `event list` output.
///
/// The interval is passed separately from the event so a recurring series can
/// print one line per expanded occurrence.
pub fn event_line(
    event: &Event,
    interval: &Interval,
    handle: &str,
    now: Timestamp,
    zone: &TimeZone,
) -> String {
    format!(
        "  {}  {}  {}",
        handle,
        event.title,
        format_interval(interval, now, zone)
    )
}

/// Render one task line for the human `list` output.
pub fn task_line(
    task: &TaskWithComments,
    handles: &HashMap<TaskId, String>,
    now: Timestamp,
    zone: &TimeZone,
) -> String {
    let checkbox = match task.state {
        TaskState::Open => "[ ]",
        TaskState::Closed => "[x]",
    };

    // fall back to the full hex id if the handle map is missing this task
    let id = handles
        .get(&task.id)
        .cloned()
        .unwrap_or_else(|| task.id.simple().to_string());
    let mut line = format!("  {}  {}", id, checkbox);
    line.push_str(&format!("  {}", task.priority.as_str()));
    line.push_str(&format!("  {}", task.title));
    if let Some(due) = &task.due {
        line.push_str(&format!("  due:{}", format_when(due, now, zone)));
    }

    let tags = task
        .tags
        .iter()
        .map(|tag| tag.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if !tags.is_empty() {
        line.push_str(&format!("  {}", tags));
    }

    line
}
