use crate::error::ServiceError;
use scry_core::error::StorageError;
use scry_core::ids::{SectionId, TaskId};
use scry_core::models::{
    Change, Color, ExternalIdentity, ExternalMetadata, Priority, Tags, Task, TaskState,
};
use scry_core::store::ArcStore;
use scry_core::sync::ack::{OperationAcknowledgment, RejectionReason};
use scry_core::sync::operations::{OPERATION_VERSION, Operation, OperationKind};
use uuid::Uuid;

mod apply;
mod automation;
mod fields;
mod handles;
mod import;
mod pipeline;
mod placement;
mod writes;

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

/// Builds the `CreateTask` wire operation for a task row.
fn create_task_operation(task: &Task) -> OperationKind {
    OperationKind::CreateTask {
        id: task.id,
        project_id: task.project_id,
        title: task.title.clone(),
        description: task.description.clone(),
        priority: task.priority,
        section_id: task.section_id,
        position: task.position.clone(),
        state: task.state,
        tags: task.tags.clone(),
        created_at: task.created_at,
        updated_at: None,
        external_identity: task.external_identity.clone(),
        external_metadata: task.external_metadata.clone().map(Box::new),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use jiff::Timestamp;
    use scry_core::models::{Automation, AutomationAction, AutomationTrigger};

    #[test]
    fn automation_references_section_in_trigger_or_actions() {
        let section_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let created_at = Timestamp::now();

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
