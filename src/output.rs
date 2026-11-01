//! Shared human and JSON output helpers.

use std::collections::HashMap;

use scry_core::ids::TaskId;
use scry_core::models::{CloseReason, Lifecycle, Section};

use crate::color;
use crate::error::AppError;
use crate::state::TaskWithNotes;

/// Print a value as compact JSON on stdout.
pub fn print_json<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
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

/// Render one task line for the human `list` output.
pub fn task_line(
    task: &TaskWithNotes,
    show_priority: bool,
    handles: &HashMap<TaskId, String>,
) -> String {
    let checkbox = match &task.lifecycle {
        Lifecycle::Open => "[ ]",
        Lifecycle::Closed { reason, .. } => match reason {
            CloseReason::Completed => "[x]",
            CloseReason::Cancelled => "[~]",
        },
    };

    // fall back to the full hex id if the handle map is missing this task
    let id = handles
        .get(&task.id)
        .cloned()
        .unwrap_or_else(|| task.id.simple().to_string());
    let mut line = format!("  {}  {}", id, checkbox);
    if show_priority {
        line.push_str(&format!("  {}", color::priority(task.priority)));
    }
    line.push_str(&format!("  {}", task.title));

    let tags = task
        .tags
        .iter()
        .map(|tag| color::tag(tag))
        .collect::<Vec<_>>()
        .join(" ");
    if !tags.is_empty() {
        line.push_str(&format!("  {}", tags));
    }

    line
}
