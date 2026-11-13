use std::cmp::Ordering;

use crate::models::{Task, TaskSortingMode};

/// Order two tasks for the project's sorting mode, breaking ties by id.
///
/// `Manual` sorts by position. `Priority` sorts by priority first and then by
/// position, so tasks sharing a priority keep their manual order.
pub fn compare_tasks(mode: TaskSortingMode, left: &Task, right: &Task) -> Ordering {
    let ordering = match mode {
        TaskSortingMode::Manual => left.position.cmp(&right.position),
        TaskSortingMode::Priority => left
            .priority
            .cmp(&right.priority)
            .then_with(|| left.position.cmp(&right.position)),
    };

    ordering.then_with(|| left.id.cmp(&right.id))
}
