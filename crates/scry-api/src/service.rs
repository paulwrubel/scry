use crate::error::ServiceError;
use chrono::Utc;
use scry_core::error::StorageError;
use scry_core::ids::{SectionId, TaskId};
use scry_core::models::{
    Automation, Change, ChangeKind, Color, Comment, Entity, EntityType, ExternalIdentity,
    ExternalMetadata, Priority, Project, Section, Tags, Task, TaskState,
};
use scry_core::position::n_keys_between;
use scry_core::store::ArcStore;
use scry_core::sync::ack::{OperationAcknowledgment, RejectionReason};
use scry_core::sync::operations::{OPERATION_VERSION, Operation, OperationKind};
use serde_json::{Map, Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

mod automation;
mod fields;
mod handles;
mod import;
mod pipeline;
mod placement;
mod writes;
use self::fields::{
    comment_field_updates, project_field_updates, section_field_updates, subject,
    task_field_updates, update_changes_locked_external_field,
};

#[cfg(test)]
mod test_helpers;

/// How an operation selects a task's section: leave it alone, clear it, use an
/// existing section, or create one by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SectionInput {
    #[default]
    Unchanged,
    NoSection,
    Existing(SectionId),
    New(String),
}

fn position_error(error: scry_core::position::PositionError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(format!(
        "failed to generate position: {error}"
    )))
}

/// Build a wire operation for a REST write. REST writes carry no client
/// timestamp (#46); the server's time is recorded on the change row.
fn new_operation(operation_id: Uuid, kind: OperationKind) -> Operation {
    Operation {
        operation_id,
        operation_version: OPERATION_VERSION,
        timestamp: None,
        kind,
    }
}

/// Turn a pipeline rejection into the error a REST write returns. Tombstoned
/// entities are reported as not found because they no longer exist.
fn rejection_error(operation_id: Uuid, reason: RejectionReason) -> ServiceError {
    let message = format!("operation {operation_id} rejected: {}", reason.as_str());
    ServiceError::Storage(match reason {
        RejectionReason::NotFound | RejectionReason::Tombstoned => StorageError::NotFound(message),
        RejectionReason::Conflict => StorageError::Conflict(message),
        RejectionReason::UnsupportedVersion => StorageError::Invalid(message),
    })
}

/// How a batch of operations treats a rejected operation.
#[derive(Debug, Clone, Copy)]
pub enum ApplyMode {
    /// Roll the whole batch back on the first rejection (REST writes).
    Atomic,
    /// Acknowledge the rejection and apply the rest of the batch (#104's `/sync`).
    PerOperation,
}

/// Whether applying an operation should evaluate the project's automations
/// after the operation takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationEvaluation {
    /// Normal user and server writes: evaluate automations after applying.
    Evaluate,
    /// Import and automation-generated actions: do not evaluate automations.
    Skip,
}

/// The acks, pulled changes, and next cursor produced by one sync round.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub acks: Vec<OperationAcknowledgment>,
    pub changes: Vec<Change>,
    pub next_seq: i64,
}

/// Input for creating a task. `SectionInput::Unchanged` means no section.
#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
    pub external_identity: Option<ExternalIdentity>,
    pub external_metadata: Option<ExternalMetadata>,
}

/// Partial task input. `None` leaves a field unchanged, including
/// `SectionInput::Unchanged` for the section; `Some("")` on `description`
/// clears it; `Some(Tags)` on `tags` sets them (an empty `Tags` clears them).
#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Tags>,
    pub section: SectionInput,
    pub state: Option<TaskState>,
    pub external_identity: Option<Option<ExternalIdentity>>,
    pub external_metadata: Option<Option<ExternalMetadata>>,
}

/// A task after a create, duplicate, or move.
#[derive(Debug, Clone)]
pub struct TaskChangeResult {
    pub task: Task,
}

/// Partial project input. `None` leaves a field unchanged; `Some("")` on
/// `description` clears it.
#[derive(Debug, Clone, Default)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// How an update changes a section's color.
#[derive(Debug, Clone, Copy)]
pub enum ColorChange {
    /// Remove the section's color.
    Clear,
    /// Set the section's color.
    Set(Color),
}

/// Partial section input. `None` leaves the name unchanged.
#[derive(Debug, Clone)]
pub struct UpdateSection {
    pub name: Option<String>,
    pub color: ColorChange,
}

/// Where to place a section relative to its siblings.
#[derive(Debug, Clone, Copy)]
pub enum SectionPlacement {
    Top,
    Bottom,
    Before(SectionId),
    After(SectionId),
}

/// Where to place a task relative to its siblings in the destination group.
#[derive(Debug, Clone, Copy)]
pub enum TaskPlacement {
    Top,
    Bottom,
    Before(TaskId),
    After(TaskId),
}

/// Frontend-agnostic, high-level operations over one project. Resolves names to
/// ids, picks defaults, computes positions, and enforces invariants before
/// delegating the raw writes to `Store`.
#[derive(Clone)]
pub struct ProjectService {
    store: ArcStore,
}

impl ProjectService {
    pub fn new(store: ArcStore) -> Self {
        Self { store }
    }

    async fn apply_one_operation(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        op: &Operation,
        automation_evaluation: AutomationEvaluation,
    ) -> Result<(OperationAcknowledgment, Option<i64>), ServiceError> {
        // 1. an op whose wire version this build cannot read is rejected on its
        // own, so the rest of the batch still applies
        if op.validate().is_err() {
            return Ok((
                OperationAcknowledgment::rejected(
                    op.operation_id,
                    RejectionReason::UnsupportedVersion,
                ),
                None,
            ));
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
            return Ok((
                OperationAcknowledgment::rejected(op.operation_id, RejectionReason::Tombstoned),
                None,
            ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let mut field_values = project_field_updates(&mut project, &op.kind);
                project.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                let deleted_at = Utc::now();

                // children come last, so reversing logs them ahead of their parent
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .get_section_by_name(*project_id, name, Some(&mut **tx))
                    .await?
                    .is_some()
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let mut field_values = section_field_updates(&mut section, &op.kind);
                section.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                        return Ok((
                            OperationAcknowledgment::rejected(
                                op.operation_id,
                                RejectionReason::Conflict,
                            ),
                            None,
                        ));
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

                    let relocated_at = Utc::now();
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
                let deleted_at = Utc::now();
                for entity in entities.iter().rev() {
                    self.store
                        .create_tombstone(*entity, deleted_at, Some(&mut **tx))
                        .await?;
                    seqs.push(
                        self.record_change(tx, op, *entity, ChangeKind::Delete, json!({}))
                            .await?,
                    );
                }

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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
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
                        return Ok((
                            OperationAcknowledgment::rejected(
                                op.operation_id,
                                RejectionReason::NotFound,
                            ),
                            None,
                        ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                if matches!(&op.kind, OperationKind::UpdateTask { .. })
                    && update_changes_locked_external_field(&task, &op.kind)
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Conflict,
                        ),
                        None,
                    ));
                }

                let before = task.clone();
                let mut field_values = task_field_updates(&mut task, &op.kind);
                task.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                let deleted_at = Utc::now();

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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                }
                if self
                    .store
                    .is_tombstoned(EntityType::Task, *task_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };

                let mut field_values = comment_field_updates(&mut comment, &op.kind);
                comment.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                let deleted_at = Utc::now();

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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                if self
                    .store
                    .is_tombstoned(EntityType::Project, *project_id, Some(&mut **tx))
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::Tombstoned,
                        ),
                        None,
                    ));
                }
                if let Some(reason) = self
                    .validate_automation(&project, trigger, actions, None, tx)
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(op.operation_id, reason),
                        None,
                    ));
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
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                let Some(project) = self
                    .store
                    .get_project_by_id(automation.project_id, Some(&mut **tx))
                    .await?
                else {
                    return Ok((
                        OperationAcknowledgment::rejected(
                            op.operation_id,
                            RejectionReason::NotFound,
                        ),
                        None,
                    ));
                };
                if let Some(reason) = self
                    .validate_automation(&project, &automation.trigger, actions, Some(*id), tx)
                    .await?
                {
                    return Ok((
                        OperationAcknowledgment::rejected(op.operation_id, reason),
                        None,
                    ));
                }

                automation.actions = actions.clone();
                automation.updated_at = (*updated_at).unwrap_or_else(Utc::now);
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
                let deleted_at = Utc::now();

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

                if existed {
                    self.store.delete_automation(*id, Some(&mut **tx)).await?;
                }
                seqs.into_iter().max()
            }
        };

        // 7. the effect is in the log, so a later replay is stopped at step 2
        self.store
            .record_applied_operation(op.operation_id, Utc::now(), Some(&mut **tx))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use scry_core::models::{AutomationAction, AutomationTrigger};

    #[test]
    fn automation_references_section_in_trigger_or_actions() {
        let section_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let created_at = Utc::now();

        let triggered = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskEntersSection { section_id },
            actions: vec![AutomationAction::CloseTask],
            created_at,
            updated_at: created_at,
        };
        assert!(triggered.references_section(section_id));
        assert!(!triggered.references_section(other_id));

        let actioned = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskCreated,
            actions: vec![AutomationAction::MoveToSection {
                section_id: Some(section_id),
            }],
            created_at,
            updated_at: created_at,
        };
        assert!(actioned.references_section(section_id));
        assert!(!actioned.references_section(other_id));

        let unrelated = Automation {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            trigger: AutomationTrigger::TaskClosed,
            actions: vec![AutomationAction::OpenTask],
            created_at,
            updated_at: created_at,
        };
        assert!(!unrelated.references_section(section_id));
    }

    #[test]
    fn automation_service_errors_have_the_expected_display_and_kind() {
        let id = Uuid::new_v4();
        let not_found = ServiceError::AutomationNotFound { id };
        assert_eq!(not_found.to_string(), format!("Automation {id} not found."));
        assert_eq!(AppError::Service(not_found).kind(), "not_found");

        let in_use = ServiceError::SectionInUseByAutomation {
            section: "Todo".to_string(),
        };
        assert_eq!(
            in_use.to_string(),
            "Section \"Todo\" is used by an automation. Remove or change that automation first."
        );
        assert_eq!(AppError::Service(in_use).kind(), "conflict");
    }
}
