use chrono::{DateTime, Utc};
use scry_core::models::{Priority, Tags, Task, TaskSortingMode, TaskState};
use scry_core::ordering::compare_tasks;
use scry_core::position::Position;
use std::cmp::Ordering;
use uuid::Uuid;

fn task(id: u128, title: &str, position: &str, priority: Priority) -> Task {
    Task {
        id: Uuid::from_u128(id),
        project_id: Uuid::nil(),
        title: title.to_string(),
        description: None,
        priority,
        section_id: None,
        position: Position::parse(position).expect("test uses a valid position"),
        state: TaskState::Open,
        tags: Tags::default(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).expect("valid timestamp"),
    }
}

fn sorted_ids(mode: TaskSortingMode, mut tasks: Vec<Task>) -> Vec<Uuid> {
    tasks.sort_by(|left, right| compare_tasks(mode, left, right));
    tasks.into_iter().map(|task| task.id).collect()
}

#[test]
fn manual_mode_orders_by_position() {
    let tasks = vec![
        task(1, "first", "a1", Priority::Medium),
        task(2, "second", "a0", Priority::Medium),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Manual, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn alphabetical_mode_orders_by_title() {
    let tasks = vec![
        task(1, "beta", "a0", Priority::Medium),
        task(2, "alpha", "a0", Priority::Medium),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Alphabetical, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn priority_mode_orders_by_priority() {
    let tasks = vec![
        task(1, "first", "a0", Priority::Minimal),
        task(2, "second", "a0", Priority::Critical),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Priority, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn ids_break_ties_in_every_mode() {
    for mode in [
        TaskSortingMode::Manual,
        TaskSortingMode::Alphabetical,
        TaskSortingMode::Priority,
    ] {
        let left = task(1, "same", "a0", Priority::Medium);
        let right = task(2, "same", "a0", Priority::Medium);

        assert_eq!(compare_tasks(mode, &left, &right), Ordering::Less);
        assert_eq!(compare_tasks(mode, &right, &left), Ordering::Greater);
    }
}
