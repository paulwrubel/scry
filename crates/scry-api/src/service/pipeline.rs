use crate::error::ServiceError;
use jiff::Timestamp;
use scry_core::error::StorageError;
use scry_core::ids::{CommentId, EventId, ProjectId, SectionId, TaskId};
use scry_core::models::{
    Change, ChangeKind, Color, Comment, Entity, Event, FieldValues, Project, Section, Task,
};
use scry_core::position::{Position, key_between};
use scry_core::sync::ack::{AcknowledgmentStatus, ApplyOutcome};
use scry_core::sync::operations::{Operation, OperationKind};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{
    ApplyMode, AutomationEvaluation, ProjectService, SectionInput, SyncResult, new_operation,
    position_error, rejection_error,
};

impl ProjectService {
    pub(super) async fn build_section(
        &self,
        project: &Project,
        name: String,
        color: Option<Color>,
    ) -> Result<(Section, Operation), ServiceError> {
        let name = self.validated_section_name(project, None, name).await?;
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        let last = sections
            .iter()
            .map(|section| &section.position)
            .max()
            .cloned();
        let position = key_between(last.as_ref(), None).map_err(position_error)?;
        let id = Uuid::new_v4();
        let created_at = Timestamp::now();
        let section = Section {
            id,
            project_id: project.id,
            name: name.clone(),
            position: position.clone(),
            color,
            created_at,
            updated_at: created_at,
        };
        let operation = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateSection {
                id,
                project_id: project.id,
                name,
                position,
                color,
                created_at,
                updated_at: None,
            },
        );
        Ok((section, operation))
    }

    /// Apply a batch of operations in one transaction. `Atomic` rolls the
    /// batch back on the first rejection so one HTTP write cannot leave partial
    /// state; `PerOperation` acknowledges the rejection and applies the rest
    /// (#104's `/sync`).
    pub async fn apply_operations(
        &self,
        ops: &[Operation],
        mode: ApplyMode,
    ) -> Result<ApplyOutcome, ServiceError> {
        if ops.is_empty() {
            return Ok(ApplyOutcome::default());
        }

        let mut tx = self.store.begin_apply().await?;
        let outcome = self
            .apply_batch(&mut tx, ops, mode, AutomationEvaluation::Evaluate)
            .await?;
        tx.commit().await.map_err(|error| {
            StorageError::Database(format!("failed to commit apply batch: {error}"))
        })?;
        Ok(outcome)
    }

    pub(super) async fn apply_batch(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        ops: &[Operation],
        mode: ApplyMode,
        automation_evaluation: AutomationEvaluation,
    ) -> Result<ApplyOutcome, ServiceError> {
        let mut acks = Vec::with_capacity(ops.len());
        let mut last_seq = None;

        for op in ops {
            let (ack, seq) = self
                .apply_one_operation(tx, op, automation_evaluation)
                .await?;
            if matches!(mode, ApplyMode::Atomic)
                && let AcknowledgmentStatus::Rejected { reason } = &ack.status
            {
                // dropping `tx` rolls the whole batch back
                return Err(rejection_error(op.operation_id, *reason));
            }
            last_seq = last_seq.max(seq);
            acks.push(ack);
        }

        Ok(ApplyOutcome { acks, last_seq })
    }

    /// Apply a pushed batch per-op, then return every change after the
    /// client's cursor plus the cursor to send next.
    ///
    /// `next_seq` is the last change returned, or `last_seen_seq` when the
    /// client is already up to date. No paging: the caller gets the whole
    /// remainder of the changes log.
    pub async fn sync(
        &self,
        ops: &[Operation],
        last_seen_seq: i64,
    ) -> Result<SyncResult, ServiceError> {
        let outcome = self.apply_operations(ops, ApplyMode::PerOperation).await?;
        let changes = self.store.changes_after(last_seen_seq, None).await?;
        let next_seq = changes
            .last()
            .map(|change| change.seq)
            .unwrap_or(last_seen_seq);

        Ok(SyncResult {
            acks: outcome.acks,
            changes,
            next_seq,
        })
    }

    /// The highest changes-log `seq`, or `0` when the log is empty.
    pub async fn max_seq(&self) -> Result<i64, ServiceError> {
        Ok(self.store.last_change_seq(None).await?)
    }

    pub(super) async fn affected_entities(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        entity: Entity,
    ) -> Result<Vec<Entity>, ServiceError> {
        let mut entities = vec![entity];

        match entity {
            Entity::Project(project_id) => {
                let sections = self
                    .store
                    .get_all_sections_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let tasks = self
                    .store
                    .get_all_tasks_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let comments = self
                    .store
                    .get_all_comments_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let automations = self
                    .store
                    .get_all_automations_by_project_id(project_id, Some(&mut **tx))
                    .await?;
                let events = self
                    .store
                    .get_all_events_by_project_id(project_id, Some(&mut **tx))
                    .await?;

                entities.extend(
                    sections
                        .into_iter()
                        .map(|section| Entity::Section(section.id)),
                );
                entities.extend(tasks.into_iter().map(|task| Entity::Task(task.id)));
                entities.extend(
                    comments
                        .into_iter()
                        .map(|comment| Entity::Comment(comment.id)),
                );
                entities.extend(
                    automations
                        .into_iter()
                        .map(|automation| Entity::Automation(automation.id)),
                );
                entities.extend(events.into_iter().map(|event| Entity::Event(event.id)));
            }
            Entity::Task(task_id) => {
                let comments = self
                    .store
                    .get_all_comments_by_task_id(task_id, Some(&mut **tx))
                    .await?;
                entities.extend(
                    comments
                        .into_iter()
                        .map(|comment| Entity::Comment(comment.id)),
                );
            }
            // a section's tasks survive and are relocated instead of tombstoned
            Entity::Section(_) | Entity::Comment(_) | Entity::Automation(_) | Entity::Event(_) => {}
        }

        Ok(entities)
    }

    pub(super) async fn record_change(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        entity: Entity,
        kind: ChangeKind,
        field_values: FieldValues,
    ) -> Result<i64, ServiceError> {
        let change = Change {
            // assigned by the database; the store ignores this placeholder
            seq: 0,
            entity_type: entity.entity_type(),
            entity_id: entity.id(),
            kind,
            operation_id: op.operation_id,
            client_timestamp: op.timestamp,
            field_values,
            created_at: Timestamp::now(),
        };

        Ok(self.store.record_change(change, Some(&mut **tx)).await?)
    }

    pub(super) async fn resolve_section(
        &self,
        project: &Project,
        input: SectionInput,
    ) -> Result<(Option<Section>, Option<Operation>), ServiceError> {
        match input {
            SectionInput::Unchanged | SectionInput::NoSection => Ok((None, None)),
            SectionInput::Existing(section_id) => Ok((
                Some(self.require_section_in_project(project, section_id).await?),
                None,
            )),
            SectionInput::New(name) => {
                let (section, operation) = self.build_section(project, name, None).await?;
                Ok((Some(section), Some(operation)))
            }
        }
    }

    pub(super) async fn validated_section_name(
        &self,
        project: &Project,
        exclude: Option<SectionId>,
        name: String,
    ) -> Result<String, ServiceError> {
        let name = name.to_string();
        if name.is_empty() {
            return Err(ServiceError::SectionNameRequired);
        }
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        if sections
            .iter()
            .any(|section| Some(section.id) != exclude && section.name == name)
        {
            return Err(ServiceError::SectionNameTaken { section: name });
        }
        Ok(name)
    }

    pub(super) async fn tasks_in_section(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(match section_id {
            Some(section_id) => {
                self.store
                    .get_all_tasks_by_section_id(section_id, None)
                    .await?
            }
            None => self
                .store
                .get_all_tasks_by_project_id(project_id, None)
                .await?
                .into_iter()
                .filter(|task| task.section_id.is_none())
                .collect(),
        })
    }

    pub(super) async fn last_task_position(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Option<Position>, ServiceError> {
        let tasks = self.tasks_in_section(project_id, section_id).await?;
        Ok(tasks.iter().map(|task| &task.position).max().cloned())
    }

    pub(super) async fn append_position(
        &self,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Position, ServiceError> {
        let last = self.last_task_position(project_id, section_id).await?;
        key_between(last.as_ref(), None).map_err(position_error)
    }

    /// The position that appends a task to the section or unsectioned group,
    /// computed inside the caller's transaction.
    pub(super) async fn append_task_position_in_transaction(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        project_id: ProjectId,
        section_id: Option<SectionId>,
    ) -> Result<Position, ServiceError> {
        let tasks = match section_id {
            Some(section_id) => {
                self.store
                    .get_all_tasks_by_section_id(section_id, Some(&mut **tx))
                    .await?
            }
            None => self
                .store
                .get_all_tasks_by_project_id(project_id, Some(&mut **tx))
                .await?
                .into_iter()
                .filter(|task| task.section_id.is_none())
                .collect(),
        };
        let last = tasks.iter().map(|task| &task.position).max().cloned();
        key_between(last.as_ref(), None).map_err(position_error)
    }

    pub(super) async fn require_project(&self, id: ProjectId) -> Result<Project, ServiceError> {
        self.store
            .get_project_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("project {id} not found")))
            })
    }

    pub(super) async fn require_task(&self, id: TaskId) -> Result<Task, ServiceError> {
        self.store.get_task_by_id(id, None).await?.ok_or_else(|| {
            ServiceError::Storage(StorageError::NotFound(format!("task {id} not found")))
        })
    }

    pub(super) async fn require_comment(&self, id: CommentId) -> Result<Comment, ServiceError> {
        self.store
            .get_comment_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("comment {id} not found")))
            })
    }

    pub(super) async fn require_event(&self, id: EventId) -> Result<Event, ServiceError> {
        self.store
            .get_event_by_id(id, None)
            .await?
            .ok_or(ServiceError::EventNotFound { id })
    }

    pub(super) async fn require_section(&self, id: SectionId) -> Result<Section, ServiceError> {
        self.store
            .get_section_by_id(id, None)
            .await?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::NotFound(format!("section {id} not found")))
            })
    }

    pub(super) async fn require_task_in_project(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<Task, ServiceError> {
        self.store
            .get_task_by_id(id, None)
            .await?
            .filter(|task| task.project_id == project.id)
            .ok_or_else(|| ServiceError::TaskNotFound {
                task_id: id,
                project: project.name.clone(),
            })
    }

    pub(super) async fn require_section_in_project(
        &self,
        project: &Project,
        id: SectionId,
    ) -> Result<Section, ServiceError> {
        self.store
            .get_section_by_id(id, None)
            .await?
            .filter(|section| section.project_id == project.id)
            .ok_or_else(|| ServiceError::SectionNotFound {
                section: id.to_string(),
                project: project.name.clone(),
            })
    }
}
