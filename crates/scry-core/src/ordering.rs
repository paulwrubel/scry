use std::cmp::Ordering;

use jiff::tz::TimeZone;

use crate::models::{Task, TaskSortingMode};

/// Order two tasks for the project's sorting mode, breaking ties by id.
///
/// `Manual` sorts by position. `Priority` sorts by priority first and then by
/// position, so tasks sharing a priority keep their manual order. `Due` sorts
/// tasks by deadline, which is the last instant each task can be finished
/// without being overdue, then by position; a task with no due date sorts last.
pub fn compare_tasks(
    mode: TaskSortingMode,
    left: &Task,
    right: &Task,
    zone: &TimeZone,
) -> Ordering {
    let ordering = match mode {
        TaskSortingMode::Manual => left.position.cmp(&right.position),
        TaskSortingMode::Priority => left
            .priority
            .cmp(&right.priority)
            .then_with(|| left.position.cmp(&right.position)),
        TaskSortingMode::Due => match (&left.due, &right.due) {
            (Some(left_due), Some(right_due)) => left_due
                .deadline(zone)
                .ok()
                .cmp(&right_due.deadline(zone).ok())
                .then_with(|| left.position.cmp(&right.position)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.position.cmp(&right.position),
        },
    };

    ordering.then_with(|| left.id.cmp(&right.id))
}
