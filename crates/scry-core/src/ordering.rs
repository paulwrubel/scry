use std::cmp::Ordering;

use crate::models::{Task, TaskSortingMode};

/// Order two tasks by the project's sorting mode, breaking ties by id.
pub fn compare_tasks(mode: TaskSortingMode, left: &Task, right: &Task) -> Ordering {
    let ordering = match mode {
        TaskSortingMode::Alphabetical => left.title.cmp(&right.title),
        TaskSortingMode::AlphabeticalCaseInsensitive => {
            left.title.to_lowercase().cmp(&right.title.to_lowercase())
        }
        TaskSortingMode::Created => left.created_at.cmp(&right.created_at),
        TaskSortingMode::Manual => left.position.cmp(&right.position),
        TaskSortingMode::Priority => left.priority.cmp(&right.priority),
    };

    ordering.then_with(|| left.id.cmp(&right.id))
}
