use crate::error::ServiceError;
use scry_core::ids::{AutomationId, TaskId};
use scry_core::models::{
    AutomationAction, AutomationTrigger, EntityType, MAX_AUTOMATION_ACTIONS, Priority, Project,
    Task, TaskState,
};
use scry_core::sync::ack::{AcknowledgmentStatus, RejectionReason};
use scry_core::sync::operations::{Operation, OperationKind};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use super::{AutomationEvaluation, ProjectService, new_operation};

/// The automation trigger represented by an applied task operation, if any.
///
/// `before` is the task before the operation and `after` is the task after it.
/// Creation fires only `TaskCreated`; state and section triggers require a
/// real change.
fn automation_trigger_for_task_change(
    op: &OperationKind,
    before: Option<&Task>,
    after: &Task,
) -> Option<AutomationTrigger> {
    match op {
        OperationKind::CreateTask { .. } => Some(AutomationTrigger::TaskCreated),
        OperationKind::UpdateTask { state: Some(_), .. } => {
            let before = before?;
            if before.state == after.state {
                None
            } else {
                Some(match after.state {
                    TaskState::Closed => AutomationTrigger::TaskClosed,
                    TaskState::Open => AutomationTrigger::TaskOpened,
                })
            }
        }
        OperationKind::MoveTask { .. } => {
            let before = before?;
            if before.section_id == after.section_id {
                None
            } else {
                after
                    .section_id
                    .map(|section_id| AutomationTrigger::TaskEntersSection { section_id })
            }
        }
        _ => None,
    }
}

/// Whether applying the action would leave the task unchanged.
fn automation_action_is_noop(action: &AutomationAction, task: &Task) -> bool {
    match action {
        AutomationAction::MoveToSection { section_id } => task.section_id == *section_id,
        AutomationAction::CloseTask => task.state == TaskState::Closed,
        AutomationAction::OpenTask => task.state == TaskState::Open,
        AutomationAction::SetPriority { priority } => task.priority == *priority,
    }
}

fn update_task_state_operation(id: TaskId, state: TaskState) -> OperationKind {
    OperationKind::UpdateTask {
        id,
        title: None,
        description: None,
        priority: None,
        tags: None,
        state: Some(state),
        external_identity: None,
        external_metadata: None,
        updated_at: None,
    }
}

fn update_task_priority_operation(id: TaskId, priority: Priority) -> OperationKind {
    OperationKind::UpdateTask {
        id,
        title: None,
        description: None,
        priority: Some(priority),
        tags: None,
        state: None,
        external_identity: None,
        external_metadata: None,
        updated_at: None,
    }
}

/// A task-created rule that moves the task is skipped when the create
/// explicitly placed it in a section.
fn create_placement_blocks_move_action(
    trigger: &AutomationTrigger,
    action: &AutomationAction,
    op: &OperationKind,
) -> bool {
    matches!(trigger, AutomationTrigger::TaskCreated)
        && matches!(action, AutomationAction::MoveToSection { .. })
        && matches!(
            op,
            OperationKind::CreateTask {
                section_id: Some(_),
                ..
            }
        )
}

impl ProjectService {
    /// Check an automation against the project's sections and sibling rules.
    ///
    /// Returns `Ok(None)` when valid, or the reason the operation should be
    /// rejected. `exclude` is the rule being updated, which is not a duplicate
    /// of itself.
    pub(super) async fn validate_automation(
        &self,
        project: &Project,
        trigger: &AutomationTrigger,
        actions: &[AutomationAction],
        exclude: Option<AutomationId>,
        conn: &mut PgConnection,
    ) -> Result<Option<RejectionReason>, ServiceError> {
        if actions.is_empty() || actions.len() > MAX_AUTOMATION_ACTIONS {
            return Ok(Some(RejectionReason::Conflict));
        }

        let section_ids = trigger
            .section_id()
            .into_iter()
            .chain(actions.iter().filter_map(AutomationAction::section_id));
        for section_id in section_ids {
            let Some(section) = self
                .store
                .get_section_by_id(section_id, Some(&mut *conn))
                .await?
            else {
                return Ok(Some(RejectionReason::NotFound));
            };
            if section.project_id != project.id {
                return Ok(Some(RejectionReason::NotFound));
            }
            if self
                .store
                .is_tombstoned(EntityType::Section, section_id, Some(&mut *conn))
                .await?
            {
                return Ok(Some(RejectionReason::Tombstoned));
            }
        }

        let rules = self
            .store
            .get_all_automations_by_project_id(project.id, Some(&mut *conn))
            .await?;
        if rules
            .iter()
            .any(|rule| Some(rule.id) != exclude && rule.trigger == *trigger)
        {
            return Ok(Some(RejectionReason::Conflict));
        }

        Ok(None)
    }

    /// Evaluate the project's rule for the trigger represented by `op`, then
    /// apply that rule's actions in order. Further automation evaluation is
    /// skipped for the generated actions.
    pub(super) async fn apply_matching_automations(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        before: Option<&Task>,
        after: &Task,
    ) -> Result<Option<i64>, ServiceError> {
        let Some(trigger) = automation_trigger_for_task_change(&op.kind, before, after) else {
            return Ok(None);
        };

        let automations = self
            .store
            .get_all_automations_by_project_id(after.project_id, Some(&mut **tx))
            .await?;
        let Some(rule) = automations.iter().find(|rule| rule.trigger == trigger) else {
            return Ok(None);
        };

        let mut current = after.clone();
        let mut max_seq = None;

        for action in &rule.actions {
            if create_placement_blocks_move_action(&trigger, action, &op.kind) {
                continue;
            }
            if automation_action_is_noop(action, &current) {
                continue;
            }

            let kind = match action {
                AutomationAction::MoveToSection { section_id } => {
                    let position = self
                        .append_task_position_in_transaction(tx, current.project_id, *section_id)
                        .await?;
                    OperationKind::MoveTask {
                        id: current.id,
                        section_id: *section_id,
                        position,
                        updated_at: None,
                    }
                }
                AutomationAction::CloseTask => {
                    update_task_state_operation(current.id, TaskState::Closed)
                }
                AutomationAction::OpenTask => {
                    update_task_state_operation(current.id, TaskState::Open)
                }
                AutomationAction::SetPriority { priority } => {
                    update_task_priority_operation(current.id, *priority)
                }
            };

            let action_op = new_operation(Uuid::new_v4(), kind);
            // Box the recursive call: the engine suppresses further automation
            // evaluation at runtime, but the compiler still needs a sized future.
            let (ack, seq) =
                Box::pin(self.apply_one_operation(tx, &action_op, AutomationEvaluation::Skip))
                    .await?;
            max_seq = max_seq.max(seq);

            if ack.status == AcknowledgmentStatus::Applied
                && let Some(task) = self
                    .store
                    .get_task_by_id(current.id, Some(&mut **tx))
                    .await?
            {
                current = task;
            }
        }

        Ok(max_seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::test_helpers::task_at;
    use chrono::Utc;
    use scry_core::ids::SectionId;
    use uuid::Uuid;

    fn create_task_op(task: &Task, section_id: Option<SectionId>) -> OperationKind {
        OperationKind::CreateTask {
            id: task.id,
            project_id: task.project_id,
            title: task.title.clone(),
            description: task.description.clone(),
            priority: task.priority,
            section_id,
            position: task.position.clone(),
            state: task.state,
            tags: task.tags.clone(),
            created_at: task.created_at,
            updated_at: None,
            external_identity: task.external_identity.clone(),
            external_metadata: task.external_metadata.clone().map(Box::new),
        }
    }

    #[test]
    fn automation_trigger_for_task_change_detects_task_transitions() {
        let id = Uuid::new_v4();
        let section_id = Uuid::new_v4();
        let created = task_at(id, None, "a0");
        assert_eq!(
            automation_trigger_for_task_change(&create_task_op(&created, None), None, &created),
            Some(AutomationTrigger::TaskCreated)
        );

        let open = task_at(id, None, "a0");
        let mut closed = open.clone();
        closed.state = TaskState::Closed;
        let close_op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: Some(TaskState::Closed),
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        };
        assert_eq!(
            automation_trigger_for_task_change(&close_op, Some(&open), &closed),
            Some(AutomationTrigger::TaskClosed)
        );

        let reopen_op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: Some(TaskState::Open),
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        };
        assert_eq!(
            automation_trigger_for_task_change(&reopen_op, Some(&closed), &open),
            Some(AutomationTrigger::TaskOpened)
        );
        assert_eq!(
            automation_trigger_for_task_change(&reopen_op, Some(&open), &open),
            None
        );

        let unsectioned = task_at(id, None, "a0");
        let sectioned = task_at(id, Some(section_id), "a0");
        let enter_op = OperationKind::MoveTask {
            id,
            section_id: Some(section_id),
            position: sectioned.position.clone(),
            updated_at: None,
        };
        assert_eq!(
            automation_trigger_for_task_change(&enter_op, Some(&unsectioned), &sectioned),
            Some(AutomationTrigger::TaskEntersSection { section_id })
        );
        assert_eq!(
            automation_trigger_for_task_change(&enter_op, Some(&sectioned), &sectioned),
            None
        );

        let leave_op = OperationKind::MoveTask {
            id,
            section_id: None,
            position: unsectioned.position.clone(),
            updated_at: None,
        };
        assert_eq!(
            automation_trigger_for_task_change(&leave_op, Some(&sectioned), &unsectioned),
            None
        );

        let comment_op = OperationKind::CreateComment {
            id: Uuid::new_v4(),
            task_id: id,
            contents: "comment".to_string(),
            created_at: Utc::now(),
            updated_at: None,
        };
        assert_eq!(
            automation_trigger_for_task_change(&comment_op, None, &created),
            None
        );
    }

    #[test]
    fn automation_action_is_noop_matches_the_current_task() {
        let section_id = Uuid::new_v4();
        let task = task_at(Uuid::new_v4(), Some(section_id), "a0");
        assert!(automation_action_is_noop(
            &AutomationAction::MoveToSection {
                section_id: Some(section_id)
            },
            &task
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::MoveToSection { section_id: None },
            &task
        ));

        let unsectioned = task_at(Uuid::new_v4(), None, "a0");
        assert!(automation_action_is_noop(
            &AutomationAction::MoveToSection { section_id: None },
            &unsectioned
        ));

        assert!(!automation_action_is_noop(
            &AutomationAction::CloseTask,
            &task
        ));
        assert!(automation_action_is_noop(
            &AutomationAction::OpenTask,
            &task
        ));
        assert!(automation_action_is_noop(
            &AutomationAction::SetPriority {
                priority: Priority::Normal
            },
            &task
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::SetPriority {
                priority: Priority::Elevated
            },
            &task
        ));

        let mut closed = task.clone();
        closed.state = TaskState::Closed;
        assert!(automation_action_is_noop(
            &AutomationAction::CloseTask,
            &closed
        ));
        assert!(!automation_action_is_noop(
            &AutomationAction::OpenTask,
            &closed
        ));
    }

    #[test]
    fn create_placement_blocks_move_action_only_for_explicit_create_sections() {
        let task = task_at(Uuid::new_v4(), Some(Uuid::new_v4()), "a0");
        let section_id = task.section_id;
        let move_action = AutomationAction::MoveToSection { section_id };

        assert!(create_placement_blocks_move_action(
            &AutomationTrigger::TaskCreated,
            &move_action,
            &create_task_op(&task, section_id)
        ));
        assert!(!create_placement_blocks_move_action(
            &AutomationTrigger::TaskCreated,
            &move_action,
            &create_task_op(&task, None)
        ));
        assert!(!create_placement_blocks_move_action(
            &AutomationTrigger::TaskClosed,
            &move_action,
            &create_task_op(&task, section_id)
        ));
    }
}
