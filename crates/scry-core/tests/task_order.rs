use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::models::{Priority, Tags, Task, TaskSortingMode, TaskState};
use scry_core::ordering::compare_tasks;
use scry_core::position::Position;
use scry_core::temporal::When;
use std::cmp::Ordering;
use uuid::Uuid;

fn test_zone() -> TimeZone {
    TimeZone::get("America/Chicago").expect("valid zone")
}

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
        closed_reason: None,
        closed_at: None,
        tags: Tags::default(),
        due: None,
        created_at: Timestamp::from_second(0).expect("valid timestamp"),
        updated_at: Timestamp::from_second(0).expect("valid timestamp"),
        external_identity: None,
        external_metadata: None,
    }
}

fn due_task(id: u128, title: &str, position: &str, due_second: i64) -> Task {
    let mut task = task(id, title, position, Priority::Normal);
    task.due = Some(When::Instant {
        timestamp: Timestamp::from_second(due_second).expect("valid timestamp"),
    });
    task
}

fn sorted_ids(mode: TaskSortingMode, mut tasks: Vec<Task>) -> Vec<Uuid> {
    let zone = test_zone();
    tasks.sort_by(|left, right| compare_tasks(mode, left, right, &zone));
    tasks.into_iter().map(|task| task.id).collect()
}

#[test]
fn manual_mode_orders_by_position() {
    let tasks = vec![
        task(1, "first", "a1", Priority::Normal),
        task(2, "second", "a0", Priority::Normal),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Manual, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn priority_mode_orders_by_priority() {
    let tasks = vec![
        task(1, "first", "a0", Priority::Normal),
        task(2, "second", "a0", Priority::Critical),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Priority, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn priority_mode_orders_same_priority_by_position() {
    let tasks = vec![
        task(1, "first", "a1", Priority::Elevated),
        task(2, "second", "a0", Priority::Elevated),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Priority, tasks),
        vec![Uuid::from_u128(2), Uuid::from_u128(1)]
    );
}

#[test]
fn priority_mode_breaks_position_ties_by_id() {
    let tasks = vec![
        task(2, "second", "a0", Priority::Elevated),
        task(1, "first", "a0", Priority::Elevated),
    ];

    assert_eq!(
        sorted_ids(TaskSortingMode::Priority, tasks),
        vec![Uuid::from_u128(1), Uuid::from_u128(2)]
    );
}

#[test]
fn ids_break_ties_in_every_mode() {
    let zone = test_zone();
    for mode in [
        TaskSortingMode::Manual,
        TaskSortingMode::Priority,
        TaskSortingMode::Due,
    ] {
        let left = task(1, "same", "a0", Priority::Normal);
        let right = task(2, "same", "a0", Priority::Normal);

        assert_eq!(compare_tasks(mode, &left, &right, &zone), Ordering::Less);
        assert_eq!(compare_tasks(mode, &right, &left, &zone), Ordering::Greater);
    }
}

#[test]
fn due_mode_orders_by_due_then_position() {
    let early = due_task(1, "early", "a1", 100);
    let late = due_task(2, "late", "a0", 200);
    let undated = task(3, "undated", "a0", Priority::Normal);

    assert_eq!(
        sorted_ids(TaskSortingMode::Due, vec![late, undated, early]),
        vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]
    );

    // equal due values keep position order even though ids would sort the other way
    let first = due_task(4, "first", "a0", 100);
    let second = due_task(3, "second", "a1", 100);

    assert_eq!(
        sorted_ids(TaskSortingMode::Due, vec![second, first]),
        vec![Uuid::from_u128(4), Uuid::from_u128(3)]
    );
}
