use crate::error::ServiceError;
use scry_core::error::StorageError;
use scry_core::models::{Section, Task};
use scry_core::position::{Position, key_between};

use super::{SectionPlacement, TaskPlacement, position_error};

pub(super) fn section_placement_position(
    sections: &[Section],
    target: &Section,
    placement: SectionPlacement,
) -> Result<Option<Position>, ServiceError> {
    let target_index = sections
        .iter()
        .position(|section| section.id == target.id)
        .ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!(
                "section {} missing from its project's ordered list",
                target.id
            )))
        })?;

    let others: Vec<&Section> = sections
        .iter()
        .filter(|section| section.id != target.id)
        .collect();

    match placement {
        SectionPlacement::Top => {
            if target_index == 0 {
                return Ok(None);
            }
            let first_other = others
                .first()
                .copied()
                .expect("a non-first section has at least one sibling");
            key_between(None, Some(&first_other.position))
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::Bottom => {
            if target_index == sections.len() - 1 {
                return Ok(None);
            }
            let last_other = others
                .last()
                .copied()
                .expect("a non-last section has at least one sibling");
            key_between(Some(&last_other.position), None)
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::Before(anchor_id) => {
            let anchor_index = sections
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            if anchor_index == target_index + 1 {
                return Ok(None);
            }
            let anchor = &sections[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let predecessor_position = others[..others_anchor_index]
                .iter()
                .rev()
                .copied()
                .find(|section| section.position < anchor.position)
                .map(|section| &section.position);
            key_between(predecessor_position, Some(&anchor.position))
                .map(Some)
                .map_err(position_error)
        }
        SectionPlacement::After(anchor_id) => {
            let anchor_index = sections
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            if target_index == anchor_index + 1 {
                return Ok(None);
            }
            let anchor = &sections[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|section| section.id == anchor_id)
                .ok_or_else(|| ServiceError::SectionNotFound {
                    section: anchor_id.to_string(),
                    project: target.project_id.to_string(),
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let successor_position = others[others_anchor_index + 1..]
                .iter()
                .copied()
                .find(|section| section.position > anchor.position)
                .map(|section| &section.position);
            key_between(Some(&anchor.position), successor_position)
                .map(Some)
                .map_err(position_error)
        }
    }
}

pub(super) fn task_placement_position(
    tasks: &[Task],
    target: &Task,
    placement: TaskPlacement,
) -> Result<Option<Position>, ServiceError> {
    let target_index = tasks.iter().position(|task| task.id == target.id);

    let others: Vec<&Task> = tasks.iter().filter(|task| task.id != target.id).collect();

    match placement {
        TaskPlacement::Top => {
            if target_index == Some(0) {
                return Ok(None);
            }
            key_between(None, others.first().map(|task| &task.position))
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::Bottom => {
            if target_index.is_some_and(|index| index + 1 == tasks.len()) {
                return Ok(None);
            }
            key_between(others.last().map(|task| &task.position), None)
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::Before(anchor_id) => {
            if anchor_id == target.id {
                return Err(ServiceError::TaskMoveAnchorItself { task: target.id });
            }
            let anchor_index = tasks
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            if let Some(target_index) = target_index
                && anchor_index == target_index + 1
            {
                return Ok(None);
            }
            let anchor = &tasks[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let predecessor_position = others[..others_anchor_index]
                .iter()
                .rev()
                .copied()
                .find(|task| task.position < anchor.position)
                .map(|task| &task.position);
            key_between(predecessor_position, Some(&anchor.position))
                .map(Some)
                .map_err(position_error)
        }
        TaskPlacement::After(anchor_id) => {
            if anchor_id == target.id {
                return Err(ServiceError::TaskMoveAnchorItself { task: target.id });
            }
            let anchor_index = tasks
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            if let Some(target_index) = target_index
                && target_index == anchor_index + 1
            {
                return Ok(None);
            }
            let anchor = &tasks[anchor_index];
            let others_anchor_index = others
                .iter()
                .position(|task| task.id == anchor_id)
                .ok_or_else(|| {
                    ServiceError::Storage(StorageError::NotFound(format!(
                        "task {anchor_id} missing from the destination group"
                    )))
                })?;
            // skip neighbors with equal keys so we don't bracket equal bounds
            let successor_position = others[others_anchor_index + 1..]
                .iter()
                .copied()
                .find(|task| task.position > anchor.position)
                .map(|task| &task.position);
            key_between(Some(&anchor.position), successor_position)
                .map(Some)
                .map_err(position_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::test_helpers::task_at;
    use chrono::Utc;
    use uuid::Uuid;

    fn section(name: &str, position: &str) -> Section {
        let created_at = Utc::now();
        Section {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            name: name.to_string(),
            position: Position::parse(position).expect("valid position"),
            color: None,
            created_at,
            updated_at: created_at,
        }
    }

    #[test]
    fn top_places_a_section_before_the_first() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];

        let result = section_placement_position(&sections, target, SectionPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top should move a non-first section");
        };
        assert!(position < sections[0].position);
    }

    #[test]
    fn top_is_a_noop_for_the_first_section() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];

        let result = section_placement_position(&sections, target, SectionPlacement::Top);

        assert!(result.expect("top placement succeeds").is_none());
    }

    #[test]
    fn bottom_places_a_section_after_the_last() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];

        let result = section_placement_position(&sections, target, SectionPlacement::Bottom);

        let Some(position) = result.expect("bottom placement computes a position") else {
            panic!("bottom should move a non-last section");
        };
        assert!(position > sections[2].position);
    }

    #[test]
    fn bottom_is_a_noop_for_the_last_section() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];

        let result = section_placement_position(&sections, target, SectionPlacement::Bottom);

        assert!(result.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn top_and_bottom_are_noops_for_the_only_section() {
        let sections = vec![section("only", "a0")];
        let target = &sections[0];

        let top = section_placement_position(&sections, target, SectionPlacement::Top);
        let bottom = section_placement_position(&sections, target, SectionPlacement::Bottom);

        assert!(top.expect("top placement succeeds").is_none());
        assert!(bottom.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn before_places_a_section_between_the_left_neighbor_and_anchor() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];
        let anchor = sections[1].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent section");
        };
        assert!(position > sections[0].position);
        assert!(position < sections[1].position);
    }

    #[test]
    fn after_places_a_section_between_the_anchor_and_right_neighbor() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent section");
        };
        assert!(position > sections[1].position);
        assert!(position < sections[2].position);
    }

    #[test]
    fn before_is_a_noop_when_already_adjacent() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        assert!(result.expect("before placement succeeds").is_none());
    }

    #[test]
    fn after_is_a_noop_when_already_adjacent() {
        let sections = vec![
            section("a0", "a0"),
            section("a1", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[2];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        assert!(result.expect("after placement succeeds").is_none());
    }

    #[test]
    fn before_skips_equal_position_neighbors() {
        let sections = vec![
            section("a0", "a0"),
            section("a1-left", "a1"),
            section("a1-right", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[3];
        let anchor = sections[2].id;

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent section");
        };
        assert!(position > sections[0].position);
        assert!(position < sections[1].position);
    }

    #[test]
    fn after_skips_equal_position_neighbors() {
        let sections = vec![
            section("a0", "a0"),
            section("a1-left", "a1"),
            section("a1-right", "a1"),
            section("a2", "a2"),
        ];
        let target = &sections[0];
        let anchor = sections[1].id;

        let result = section_placement_position(&sections, target, SectionPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent section");
        };
        assert!(position > sections[1].position);
        assert!(position < sections[3].position);
    }

    #[test]
    fn before_with_an_unknown_anchor_is_not_found() {
        let sections = vec![section("a0", "a0"), section("a1", "a1")];
        let target = &sections[1];
        let anchor = Uuid::new_v4();

        let result =
            section_placement_position(&sections, target, SectionPlacement::Before(anchor));

        assert!(matches!(result, Err(ServiceError::SectionNotFound { .. })));
    }

    #[test]
    fn task_top_places_before_the_first() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];

        let result = task_placement_position(&tasks, target, TaskPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top should move a non-first task");
        };
        assert!(position < tasks[0].position);
    }

    #[test]
    fn task_top_is_a_noop_for_the_first_task() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];

        let result = task_placement_position(&tasks, target, TaskPlacement::Top);

        assert!(result.expect("top placement succeeds").is_none());
    }

    #[test]
    fn task_bottom_places_after_the_last() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];

        let result = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        let Some(position) = result.expect("bottom placement computes a position") else {
            panic!("bottom should move a non-last task");
        };
        assert!(position > tasks[2].position);
    }

    #[test]
    fn task_bottom_is_a_noop_for_the_last_task() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];

        let result = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        assert!(result.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn task_top_and_bottom_are_noops_for_the_only_task() {
        let tasks = vec![task_at(Uuid::new_v4(), None, "a0")];
        let target = &tasks[0];

        let top = task_placement_position(&tasks, target, TaskPlacement::Top);
        let bottom = task_placement_position(&tasks, target, TaskPlacement::Bottom);

        assert!(top.expect("top placement succeeds").is_none());
        assert!(bottom.expect("bottom placement succeeds").is_none());
    }

    #[test]
    fn task_before_places_between_the_left_neighbor_and_anchor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent task");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_after_places_between_the_anchor_and_right_neighbor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent task");
        };
        assert!(position > tasks[1].position);
        assert!(position < tasks[2].position);
    }

    #[test]
    fn task_before_is_a_noop_when_already_adjacent() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(result.expect("before placement succeeds").is_none());
    }

    #[test]
    fn task_after_is_a_noop_when_already_adjacent() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[2];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        assert!(result.expect("after placement succeeds").is_none());
    }

    #[test]
    fn task_before_skips_equal_position_neighbors() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[3];
        let anchor = tasks[2].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before should move a non-adjacent task");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_after_skips_equal_position_neighbors() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a1"),
            task_at(Uuid::new_v4(), None, "a2"),
        ];
        let target = &tasks[0];
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, target, TaskPlacement::After(anchor));

        let Some(position) = result.expect("after placement computes a position") else {
            panic!("after should move a non-adjacent task");
        };
        assert!(position > tasks[1].position);
        assert!(position < tasks[3].position);
    }

    #[test]
    fn task_top_into_a_different_group_is_not_a_noop() {
        let destination = Uuid::new_v4();
        let tasks = vec![
            task_at(Uuid::new_v4(), Some(destination), "a1"),
            task_at(Uuid::new_v4(), Some(destination), "a2"),
        ];
        let target = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");

        let result = task_placement_position(&tasks, &target, TaskPlacement::Top);

        let Some(position) = result.expect("top placement computes a position") else {
            panic!("top into a different group must not be a noop");
        };
        assert!(position < tasks[0].position);
    }

    #[test]
    fn task_before_into_a_different_group_places_before_the_anchor() {
        let destination = Uuid::new_v4();
        let tasks = vec![
            task_at(Uuid::new_v4(), Some(destination), "a1"),
            task_at(Uuid::new_v4(), Some(destination), "a2"),
        ];
        let target = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");
        let anchor = tasks[1].id;

        let result = task_placement_position(&tasks, &target, TaskPlacement::Before(anchor));

        let Some(position) = result.expect("before placement computes a position") else {
            panic!("before into a different group must not be a noop");
        };
        assert!(position > tasks[0].position);
        assert!(position < tasks[1].position);
    }

    #[test]
    fn task_before_rejects_the_task_itself_as_anchor() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
        ];
        let target = &tasks[1];
        let anchor = target.id;

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(matches!(
            result,
            Err(ServiceError::TaskMoveAnchorItself { .. })
        ));
    }

    #[test]
    fn task_before_with_an_unknown_anchor_is_not_found() {
        let tasks = vec![
            task_at(Uuid::new_v4(), None, "a0"),
            task_at(Uuid::new_v4(), None, "a1"),
        ];
        let target = &tasks[1];
        let anchor = Uuid::new_v4();

        let result = task_placement_position(&tasks, target, TaskPlacement::Before(anchor));

        assert!(matches!(
            result,
            Err(ServiceError::Storage(StorageError::NotFound(_)))
        ));
    }
}
