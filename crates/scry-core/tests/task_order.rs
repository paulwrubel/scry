mod common;

use common::{FixtureCase, load_area};
use scry_core::models::{Task, TaskSortingMode};
use scry_core::ordering::compare_tasks;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SortInput {
    mode: TaskSortingMode,
    tasks: Vec<Task>,
}

#[test]
fn task_order_matches_the_shared_fixtures() {
    let file = load_area("task-order");

    for case in &file.cases {
        match case.kind.as_str() {
            "sort" => check_sort(case),
            other => panic!("case `{}` uses unknown fixture kind `{other}`", case.name),
        }
    }
}

fn check_sort(case: &FixtureCase) {
    let input: SortInput = serde_json::from_value(case.input.clone())
        .unwrap_or_else(|error| panic!("case `{}` has invalid input: {error}", case.name));

    let mut tasks = input.tasks;
    tasks.sort_by(|left, right| compare_tasks(input.mode, left, right));

    let actual: Vec<String> = tasks.iter().map(|task| task.id.to_string()).collect();
    let expected = case
        .expected
        .get("ids")
        .cloned()
        .unwrap_or_else(|| panic!("case `{}` is missing `ids`", case.name));
    let expected: Vec<String> = serde_json::from_value(expected)
        .unwrap_or_else(|error| panic!("case `{}` has invalid `ids`: {error}", case.name));

    assert_eq!(actual, expected);
}
