use super::fields::{
    comment_field_updates, project_field_updates, section_field_updates, subject,
    task_field_updates, update_changes_locked_external_field,
};
use super::{AutomationEvaluation, ProjectService, position_error};
use crate::error::ServiceError;
use jiff::Timestamp;
use scry_core::models::{
    Automation, ChangeKind, Comment, Entity, EntityType, Project, Section, Task,
};
use scry_core::position::n_keys_between;
use scry_core::sync::ack::{OperationAcknowledgment, RejectionReason};
use scry_core::sync::operations::{Operation, OperationKind};
use serde_json::{Map, Value, json};
use sqlx::{Postgres, Transaction};

/// A rejection produces no change-log row, so it has no seq.
fn rejected(
    op: &Operation,
    reason: RejectionReason,
) -> Result<(OperationAcknowledgment, Option<i64>), ServiceError> {
    Ok((OperationAcknowledgment::rejected(op.operation_id, reason), None))
}

impl ProjectService {
    pub(super) async fn apply_one_operation(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        automation_evaluation: AutomationEvaluation,
    ) -> Result<(OperationAcknowledgment, Option<i64>), ServiceError> {
        // 1. an op whose wire version this build cannot read is rejected on its
        // own, so the rest of the batch still applies
        if op.validate().is_err() {
            return rejected(op, RejectionReason::UnsupportedVersion);
        }

        // 2. a re-transmission: acknowledge it without a second effect
        if self
            .store
            .is_operation_applied(op.operation_id, Some(&mut **tx))
            .await?
        {
            return Ok((OperationAcknowledgment::applied(op.operation_id), None));
        }

        // 3. an edit to a deleted entity loses; deletes are permanent
        let subject = subject(&op.kind);
        if self
            .store
            .is_tombstoned(subject.entity_type(), subject.id(), Some(&mut **tx))
            .await?
        {
            return rejected(op, RejectionReason::Tombstoned);
        }

        // 4-6. apply the fields the op names, write through the store, then log
        // one change per affected entity; deletes fan out via affected_entities
        let mut task_transition: Option<(Option<Task>, Task)> = None;
        let mut seq = match &op.kind {
            OperationKind::CreateProject {
                id,
                name,
                description,
                created_at,
                updated_at,
            } => {
                if self
                    .store
                    .get_project_by_name(name, Some(&mut **tx))
                    .await?
                    .is_some()
                {
                    return rejected(op, RejectionReason::Conflict);
                }

                let project = Project {
                    id: *id,
                    name: name.clone(),
                    description: description
                        .clone()
                        .filter(|description| !description.is_empty()),
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                };
                self.store
                    .create_project(project.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&project).expect("a project serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Project(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateProject { id, updated_at, .. } => {
                let Some(mut project) = self.store.get_project_by_id(*id, Some(&mut **tx)).await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };

                let mut field_values = project_field_updates(&mut project, &op.kind);
                project.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                field_values["updated_at"] = json!(project.updated_at);
                self.store.update_project(project, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Project(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteProject { id } => {
                let existed = self
                    .store
                    .get_project_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Project(*id)).await?;
                let seqs = self.record_deletes(tx, op, &entities).await?;

                if existed {
                    self.store
                        .delete_project_by_id(*id, Some(&mut **tx))
                        .await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateSection {
                id,
                project_id,
                name,
                position,
                color,
                created_at,
                updated_at,
            } => {
                if self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return rejected(op, RejectionReason::NotFound);
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return rejected(op, RejectionReason::Tombstoned);
                }
                if self
                    .store
                    .get_section_by_name(*project_id, name, Some(&mut **tx))
                    .await?
                    .is_some()
                {
                    return rejected(op, RejectionReason::Conflict);
                }

                let section = Section {
                    id: *id,
                    project_id: *project_id,
                    name: name.clone(),
                    position: position.clone(),
                    color: *color,
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                };
                self.store
                    .create_section(section.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&section).expect("a section serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateSection { id, updated_at, .. } => {
                let Some(mut section) = self.store.get_section_by_id(*id, Some(&mut **tx)).await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                field_values["updated_at"] = json!(section.updated_at);
                self.store.update_section(section, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::MoveSection { id, updated_at, .. } => {
                let Some(mut section) = self.store.get_section_by_id(*id, Some(&mut **tx)).await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                field_values["updated_at"] = json!(section.updated_at);
                self.store.update_section(section, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Section(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteSection { id } => {
                let section = self.store.get_section_by_id(*id, Some(&mut **tx)).await?;
                let mut seqs = Vec::new();

                // a section's tasks survive its deletion, so they are relocated
                // to No section and logged as updates rather than tombstoned
                if let Some(section) = &section {
                    let automations = self
                        .store
                        .get_all_automations_by_project_id(section.project_id, Some(&mut **tx))
                        .await?;
                    if automations
                        .iter()
                        .any(|automation| automation.references_section(section.id))
                    {
                        return rejected(op, RejectionReason::Conflict);
                    }

                    let tasks = self
                        .store
                        .get_all_tasks_by_section_id(section.id, Some(&mut **tx))
                        .await?;
                    let unsectioned = self
                        .store
                        .get_all_tasks_by_project_id(section.project_id, Some(&mut **tx))
                        .await?
                        .into_iter()
                        .filter(|task| task.section_id.is_none())
                        .collect::<Vec<_>>();
                    let last = unsectioned.iter().map(|task| &task.position).max().cloned();
                    let positions =
                        n_keys_between(last.as_ref(), None, tasks.len()).map_err(position_error)?;

                    let relocated_at = Timestamp::now();
                    for (mut task, position) in tasks.into_iter().zip(positions) {
                        task.section_id = None;
                        task.position = position.clone();
                        task.updated_at = relocated_at;
                        let task_id = task.id;
                        self.store.update_task(task, Some(&mut **tx)).await?;

                        let mut field_values = Map::new();
                        field_values.insert("section_id".into(), Value::Null);
                        field_values.insert("position".into(), json!(position));
                        field_values.insert("updated_at".into(), json!(relocated_at));
                        seqs.push(
                            self.record_change(
                                tx,
                                op,
                                Entity::Task(task_id),
                                ChangeKind::Update,
                                Value::Object(field_values),
                            )
                            .await?,
                        );
                    }
                }

                let entities = self.affected_entities(tx, Entity::Section(*id)).await?;
                seqs.extend(self.record_deletes(tx, op, &entities).await?);

                if section.is_some() {
                    self.store.delete_section(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateTask {
                id,
                project_id,
                title,
                description,
                priority,
                section_id,
                position,
                state,
                tags,
                created_at,
                updated_at,
                external_identity,
                external_metadata,
            } => {
                if self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return rejected(op, RejectionReason::NotFound);
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return rejected(op, RejectionReason::Tombstoned);
                }
                if let Some(section_id) = section_id {
                    let belongs = self
                        .store
                        .get_section_by_id(*section_id, Some(&mut **tx))
                        .await?
                        .is_some_and(|section| section.project_id == *project_id);
                    let tombstoned = self
                        .store
                        .is_tombstoned(EntityType::Section, *section_id, Some(&mut **tx))
                        .await?;
                    if !belongs || tombstoned {
                        return rejected(op, RejectionReason::NotFound);
                    }
                }

                let task = Task {
                    id: *id,
                    project_id: *project_id,
                    title: title.clone(),
                    description: description
                        .clone()
                        .filter(|description| !description.is_empty()),
                    priority: *priority,
                    section_id: *section_id,
                    position: position.clone(),
                    state: *state,
                    tags: tags.clone(),
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                    external_identity: external_identity.clone(),
                    external_metadata: external_metadata.as_deref().cloned(),
                };
                task_transition = Some((None, task.clone()));
                self.store
                    .create_task(task.clone(), Some(&mut **tx))
                    .await?;

                let field_values = serde_json::to_value(&task).expect("a task serializes to JSON");
                Some(
                    self.record_change(tx, op, Entity::Task(*id), ChangeKind::Create, field_values)
                        .await?,
                )
            }
            OperationKind::UpdateTask { id, updated_at, .. }
            | OperationKind::MoveTask { id, updated_at, .. } => {
                let Some(mut task) = self.store.get_task_by_id(*id, Some(&mut **tx)).await? else {
                    return rejected(op, RejectionReason::NotFound);
                };

                if matches!(&op.kind, OperationKind::UpdateTask { .. })
                    && update_changes_locked_external_field(&task, &op.kind)
                {
                    return rejected(op, RejectionReason::Conflict);
                }

                let before = task.clone();
                let mut field_values = task_field_updates(&mut task, &op.kind);
                task.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                field_values["updated_at"] = json!(task.updated_at);
                task_transition = Some((Some(before), task.clone()));
                self.store.update_task(task, Some(&mut **tx)).await?;
                Some(
                    self.record_change(tx, op, Entity::Task(*id), ChangeKind::Update, field_values)
                        .await?,
                )
            }
            OperationKind::DeleteTask { id } => {
                let existed = self
                    .store
                    .get_task_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Task(*id)).await?;
                let seqs = self.record_deletes(tx, op, &entities).await?;

                if existed {
                    self.store.delete_task(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateComment {
                id,
                task_id,
                contents,
                created_at,
                updated_at,
            } => {
                if self
                    .store
                    .get_task_by_id(*task_id, Some(&mut **tx))
                    .await?
                    .is_none()
                {
                    return rejected(op, RejectionReason::NotFound);
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Task, *task_id, Some(&mut **tx))
                    .await?
                {
                    return rejected(op, RejectionReason::Tombstoned);
                }

                let comment = Comment {
                    id: *id,
                    task_id: *task_id,
                    contents: contents.clone(),
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                };
                self.store
                    .create_comment(comment.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&comment).expect("a comment serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Comment(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateComment { id, updated_at, .. } => {
                let Some(mut comment) = self.store.get_comment_by_id(*id, Some(&mut **tx)).await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };

                let mut field_values = comment_field_updates(&mut comment, &op.kind);
                comment.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                field_values["updated_at"] = json!(comment.updated_at);
                self.store.update_comment(comment, Some(&mut **tx)).await?;
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Comment(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteComment { id } => {
                let existed = self
                    .store
                    .get_comment_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Comment(*id)).await?;
                let seqs = self.record_deletes(tx, op, &entities).await?;

                if existed {
                    self.store.delete_comment(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
            OperationKind::CreateAutomation {
                id,
                project_id,
                trigger,
                actions,
                created_at,
                updated_at,
            } => {
                let Some(project) = self
                    .store
                    .get_project_by_id(*project_id, Some(&mut **tx))
                    .await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return rejected(op, RejectionReason::Tombstoned);
                }
                if let Some(reason) = self
                    .validate_automation(&project, trigger, actions, None, tx)
                    .await?
                {
                    return rejected(op, reason);
                }

                let automation = Automation {
                    id: *id,
                    project_id: *project_id,
                    trigger: trigger.clone(),
                    actions: actions.clone(),
                    created_at: *created_at,
                    updated_at: (*updated_at).unwrap_or(*created_at),
                };
                self.store
                    .create_automation(automation.clone(), Some(&mut **tx))
                    .await?;

                let field_values =
                    serde_json::to_value(&automation).expect("an automation serializes to JSON");
                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Automation(*id),
                        ChangeKind::Create,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::UpdateAutomation {
                id,
                actions,
                updated_at,
            } => {
                let Some(mut automation) = self
                    .store
                    .get_automation_by_id(*id, Some(&mut **tx))
                    .await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };
                let Some(project) = self
                    .store
                    .get_project_by_id(automation.project_id, Some(&mut **tx))
                    .await?
                else {
                    return rejected(op, RejectionReason::NotFound);
                };
                if let Some(reason) = self
                    .validate_automation(&project, &automation.trigger, actions, Some(*id), tx)
                    .await?
                {
                    return rejected(op, reason);
                }

                automation.actions = actions.clone();
                automation.updated_at = (*updated_at).unwrap_or_else(Timestamp::now);
                let field_values =
                    json!({ "actions": actions, "updated_at": automation.updated_at });
                self.store
                    .update_automation(automation, Some(&mut **tx))
                    .await?;

                Some(
                    self.record_change(
                        tx,
                        op,
                        Entity::Automation(*id),
                        ChangeKind::Update,
                        field_values,
                    )
                    .await?,
                )
            }
            OperationKind::DeleteAutomation { id } => {
                let existed = self
                    .store
                    .get_automation_by_id(*id, Some(&mut **tx))
                    .await?
                    .is_some();
                let entities = self.affected_entities(tx, Entity::Automation(*id)).await?;
                let seqs = self.record_deletes(tx, op, &entities).await?;

                if existed {
                    self.store.delete_automation(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
        };

        // 7. the effect is in the log, so a later replay is stopped at step 2
        self.store
            .record_applied_operation(op.operation_id, Timestamp::now(), Some(&mut **tx))
            .await?;

        if automation_evaluation == AutomationEvaluation::Evaluate
            && let Some((before, after)) = &task_transition
        {
            seq = seq.max(
                self.apply_matching_automations(tx, op, before.as_ref(), after)
                    .await?,
            );
        }

        Ok((OperationAcknowledgment::applied(op.operation_id), seq))
    }

    /// Tombstone each affected entity and log one delete change per entity.
    /// Children come last, so reversing logs them ahead of their parent.
    async fn record_deletes(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        entities: &[Entity],
    ) -> Result<Vec<i64>, ServiceError> {
        let deleted_at = Timestamp::now();
        let mut seqs = Vec::with_capacity(entities.len());
        for entity in entities.iter().rev() {
            self.store
                .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                .await?;
            seqs.push(
                self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                    .await?,
            );
        }
        Ok(seqs)
    }
}
