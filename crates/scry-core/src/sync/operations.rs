use crate::ids::{AutomationId, CommentId, EventId, ProjectId, SectionId, TaskId};
use crate::models::{
    AutomationAction, AutomationTrigger, ClosedReason, Color, ExternalIdentity, ExternalMetadata,
    Priority, Tags, TaskState,
};
use crate::position::Position;
use crate::recurrence::RRule;
use crate::temporal::{Interval, When};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// The version of the operation wire format. Bump by one when the shape changes; the
/// server accepts the current version and one previous major version.
pub const OPERATION_VERSION: u16 = 1;

/// Whether `version` is accepted by a peer whose current version is `current`.
/// Only the current and the immediately preceding version are accepted, so a
/// gap of two or more is rejected. The arithmetic avoids underflow at 0.
pub const fn operation_version_supported_at(version: u16, current: u16) -> bool {
    version == current || (current > 1 && version == current - 1)
}

/// Whether this build accepts `version`.
pub fn operation_version_supported(version: u16) -> bool {
    operation_version_supported_at(version, OPERATION_VERSION)
}

/// The lowest version this build accepts: one behind the current version, or
/// the current version when there is no previous one to accept.
pub fn min_supported_operation_version() -> u16 {
    if OPERATION_VERSION > 1 {
        OPERATION_VERSION - 1
    } else {
        OPERATION_VERSION
    }
}

/// A typed rejection of an operation whose wire format this build cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    UnsupportedOperationVersion {
        found: u16,
        current: u16,
        min_accepted: u16,
    },
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOperationVersion {
                found,
                current,
                min_accepted,
            } => write!(
                f,
                "unsupported operation version {found} (this server accepts {min_accepted}..={current})"
            ),
        }
    }
}

impl std::error::Error for OperationError {}

/// One sync operation as it travels over the wire.
///
/// `timestamp` is a nullable client-supplied wall-clock time that the server
/// records but never compares; REST and import operations leave it null. `operation_version`
/// is validated by [`Operation::validate`] rather than during deserialization so one
/// unsupported operation can still be acknowledged on its own instead of failing the
/// whole batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Operation {
    pub operation_id: Uuid,
    pub operation_version: u16,
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub timestamp: Option<Timestamp>,
    #[serde(flatten)]
    pub kind: OperationKind,
}

impl Operation {
    pub fn validate(&self) -> Result<(), OperationError> {
        if operation_version_supported(self.operation_version) {
            Ok(())
        } else {
            Err(OperationError::UnsupportedOperationVersion {
                found: self.operation_version,
                current: OPERATION_VERSION,
                min_accepted: min_supported_operation_version(),
            })
        }
    }
}

/// A section color update. `None` on [`OperationKind::UpdateSection::color`] leaves the
/// color unchanged; [`ColorChange::Clear`] removes it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ColorChange {
    Clear,
    Set(Color),
}

/// The operation vocabulary: one variant per `ProjectService` mutation, entity at a
/// time. Section merge is deliberately absent because it is a server-side rule
/// that emits ordinary operations. Moves stay separate so moving a task never
/// rewrites its other fields.
///
/// Create variants carry the client-generated id and `created_at` so
/// offline-created entities keep stable identity and Created ordering;
/// server-side REST creates fill those fields before building the operation.
/// Create, update, and move variants may also carry the client's edit time as
/// `updated_at`; the server stamps its own clock when it is absent, and creates
/// fall back to the operation's `created_at`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationKind {
    CreateProject {
        id: ProjectId,
        name: String,
        description: Option<String>,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    /// `None` leaves a field unchanged. `description` distinguishes all three
    /// states: `None` leaves it, `Some(None)` clears it, `Some(Some(text))`
    /// sets it.
    UpdateProject {
        id: ProjectId,
        name: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        description: Option<Option<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteProject {
        id: ProjectId,
    },
    CreateSection {
        id: SectionId,
        project_id: ProjectId,
        name: String,
        position: Position,
        color: Option<Color>,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    UpdateSection {
        id: SectionId,
        name: Option<String>,
        color: Option<ColorChange>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    MoveSection {
        id: SectionId,
        position: Position,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteSection {
        id: SectionId,
    },
    CreateTask {
        id: TaskId,
        project_id: ProjectId,
        title: String,
        description: Option<String>,
        priority: Priority,
        section_id: Option<SectionId>,
        position: Position,
        #[serde(default)]
        state: TaskState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        closed_reason: Option<ClosedReason>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        closed_at: Option<Timestamp>,
        tags: Tags,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        due: Option<When>,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_identity: Option<ExternalIdentity>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_metadata: Option<Box<ExternalMetadata>>,
    },
    /// Covers title, description, priority, tags, state, due, external identity,
    /// and external metadata: section and position belong to
    /// [`OperationKind::MoveTask`].
    /// `description` distinguishes all three states: `None` leaves it,
    /// `Some(None)` clears it, `Some(Some(text))` sets it.
    /// `Some(tags)` sets them, empty `Tags` clears them, `None` leaves them.
    /// `due` likewise: `None` leaves it, `Some(None)` clears it, `Some(Some(when))` sets it.
    /// `state: Some(Closed)` records `closed_reason` (defaulting to `completed`
    /// when absent) and `closed_at` (the operation's edit time when absent);
    /// `state: Some(Open)` clears both. An explicit `closed_reason` or
    /// `closed_at` is applied only while the resulting task is closed, which is
    /// how snapshot import restores them; on an open task they are ignored and
    /// the fields are cleared. `None` leaves them.
    UpdateTask {
        id: TaskId,
        title: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        description: Option<Option<String>>,
        priority: Option<Priority>,
        tags: Option<Tags>,
        state: Option<TaskState>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "ClosedReason | null"))]
        closed_reason: Option<Option<ClosedReason>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        closed_at: Option<Option<Timestamp>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "When | null"))]
        due: Option<Option<When>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "ExternalIdentity | null"))]
        external_identity: Option<Option<ExternalIdentity>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "ExternalMetadata | null"))]
        external_metadata: Option<Option<Box<ExternalMetadata>>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    /// `section_id` and `position` are always set together; `None` means the
    /// task moves to No section.
    MoveTask {
        id: TaskId,
        section_id: Option<SectionId>,
        position: Position,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteTask {
        id: TaskId,
    },
    CreateComment {
        id: CommentId,
        task_id: TaskId,
        contents: String,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    UpdateComment {
        id: CommentId,
        contents: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteComment {
        id: CommentId,
    },
    CreateAutomation {
        id: AutomationId,
        project_id: ProjectId,
        trigger: AutomationTrigger,
        actions: Vec<AutomationAction>,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    UpdateAutomation {
        id: AutomationId,
        actions: Vec<AutomationAction>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteAutomation {
        id: AutomationId,
    },
    CreateEvent {
        id: EventId,
        project_id: ProjectId,
        title: String,
        description: Option<String>,
        interval: Interval,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recurrence: Option<RRule>,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        created_at: Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    /// `description` distinguishes all three states: `None` leaves it,
    /// `Some(None)` clears it, `Some(Some(text))` sets it.
    UpdateEvent {
        id: EventId,
        title: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        description: Option<Option<String>>,
        interval: Option<Interval>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::sync::double_option"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<RRule>"))]
        recurrence: Option<Option<RRule>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        updated_at: Option<Timestamp>,
    },
    DeleteEvent {
        id: EventId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn update_task_op(description: Option<&str>, include: bool) -> serde_json::Value {
        let mut value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "update_task",
            "id": Uuid::new_v4(),
        });
        if include {
            value["description"] = match description {
                Some(text) => json!(text),
                None => json!(null),
            };
        }
        value
    }

    fn description_of(operation: &Operation) -> &Option<Option<String>> {
        match &operation.kind {
            OperationKind::UpdateTask { description, .. } => description,
            other => panic!("expected UpdateTask, got {other:?}"),
        }
    }

    #[test]
    fn absent_description_is_unchanged() {
        let operation: Operation = serde_json::from_value(update_task_op(None, false)).unwrap();
        assert_eq!(description_of(&operation), &None);
    }

    #[test]
    fn null_description_clears() {
        let operation: Operation = serde_json::from_value(update_task_op(None, true)).unwrap();
        assert_eq!(description_of(&operation), &Some(None));
    }

    #[test]
    fn string_description_sets() {
        let operation: Operation =
            serde_json::from_value(update_task_op(Some("hello"), true)).unwrap();
        assert_eq!(description_of(&operation), &Some(Some("hello".to_string())));
    }

    #[test]
    fn description_serializes_in_three_states() {
        let mut operation: Operation = serde_json::from_value(update_task_op(None, false)).unwrap();

        let absent = serde_json::to_value(&operation).unwrap();
        assert!(absent.get("description").is_none());

        if let OperationKind::UpdateTask { description, .. } = &mut operation.kind {
            *description = Some(None);
        }
        assert_eq!(
            serde_json::to_value(&operation).unwrap().get("description"),
            Some(&json!(null))
        );

        if let OperationKind::UpdateTask { description, .. } = &mut operation.kind {
            *description = Some(Some("hello".to_string()));
        }
        assert_eq!(
            serde_json::to_value(&operation).unwrap().get("description"),
            Some(&json!("hello"))
        );
    }

    #[test]
    fn absent_external_task_fields_are_unchanged() {
        let operation: Operation = serde_json::from_value(update_task_op(None, false)).unwrap();
        let OperationKind::UpdateTask {
            external_identity,
            external_metadata,
            ..
        } = &operation.kind
        else {
            panic!("expected UpdateTask");
        };
        assert!(external_identity.is_none());
        assert!(external_metadata.is_none());
    }

    #[test]
    fn null_external_task_fields_clear() {
        let mut value = update_task_op(None, false);
        value["external_identity"] = json!(null);
        value["external_metadata"] = json!(null);

        let operation: Operation = serde_json::from_value(value).unwrap();
        let OperationKind::UpdateTask {
            external_identity,
            external_metadata,
            ..
        } = &operation.kind
        else {
            panic!("expected UpdateTask");
        };
        assert_eq!(external_identity, &Some(None));
        assert_eq!(external_metadata, &Some(None));
    }

    #[test]
    fn object_external_task_fields_set() {
        let mut value = update_task_op(None, false);
        value["external_identity"] = json!({
            "provider": "github",
            "owner": "acme",
            "repo": "widgets",
            "number": 42
        });
        value["external_metadata"] = json!({
            "provider": "github",
            "title": "Fix login",
            "description": "Users cannot sign in",
            "kind": "issue",
            "state": "open",
            "reason": "reopened",
            "draft": false,
            "updated_at": "2026-01-01T00:00:00Z",
            "error": null
        });

        let operation: Operation = serde_json::from_value(value).unwrap();
        let OperationKind::UpdateTask {
            external_identity,
            external_metadata,
            ..
        } = &operation.kind
        else {
            panic!("expected UpdateTask");
        };
        assert_eq!(
            external_identity,
            &Some(Some(ExternalIdentity::Github {
                owner: "acme".to_string(),
                repo: "widgets".to_string(),
                number: 42,
            }))
        );
        let Some(Some(metadata)) = external_metadata else {
            panic!("expected external metadata");
        };
        let ExternalMetadata::Github {
            title: Some(title),
            state: Some(state),
            draft: Some(false),
            ..
        } = metadata.as_ref()
        else {
            panic!("expected GitHub metadata");
        };
        assert_eq!(title.as_str(), "Fix login");
        assert_eq!(state.as_str(), "open");
    }

    #[test]
    fn external_task_fields_serialize_in_three_states() {
        let mut operation: Operation = serde_json::from_value(update_task_op(None, false)).unwrap();

        let absent = serde_json::to_value(&operation).unwrap();
        assert!(absent.get("external_identity").is_none());
        assert!(absent.get("external_metadata").is_none());

        if let OperationKind::UpdateTask {
            external_identity,
            external_metadata,
            ..
        } = &mut operation.kind
        {
            *external_identity = Some(None);
            *external_metadata = Some(None);
        }
        let cleared = serde_json::to_value(&operation).unwrap();
        assert_eq!(cleared.get("external_identity"), Some(&json!(null)));
        assert_eq!(cleared.get("external_metadata"), Some(&json!(null)));

        let identity = ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 42,
        };
        let metadata = ExternalMetadata::Github {
            title: Some("Fix login".to_string()),
            description: None,
            kind: Some("issue".to_string()),
            state: Some("open".to_string()),
            reason: None,
            draft: Some(false),
            updated_at: None,
            synced_at: None,
            error: None,
        };
        if let OperationKind::UpdateTask {
            external_identity,
            external_metadata,
            ..
        } = &mut operation.kind
        {
            *external_identity = Some(Some(identity.clone()));
            *external_metadata = Some(Some(Box::new(metadata.clone())));
        }
        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("external_identity"), Some(&json!(identity)));
        assert_eq!(serialized.get("external_metadata"), Some(&json!(metadata)));
    }

    #[test]
    fn create_automation_round_trips() {
        let section_id = Uuid::new_v4();
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "create_automation",
            "id": Uuid::new_v4(),
            "project_id": Uuid::new_v4(),
            "trigger": { "kind": "task_enters_section", "section_id": section_id },
            "actions": [{ "kind": "move_to_section", "section_id": section_id }],
            "created_at": "2026-01-01T00:00:00Z"
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::CreateAutomation {
            trigger, actions, ..
        } = &operation.kind
        else {
            panic!("expected CreateAutomation");
        };
        assert_eq!(
            trigger,
            &AutomationTrigger::TaskEntersSection { section_id }
        );
        assert_eq!(
            actions,
            &[AutomationAction::MoveToSection {
                section_id: Some(section_id)
            }]
        );

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("kind"), Some(&json!("create_automation")));
        assert_eq!(serialized, value);
    }

    #[test]
    fn update_automation_round_trips() {
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "update_automation",
            "id": Uuid::new_v4(),
            "actions": [{ "kind": "set_priority", "priority": "elevated" }]
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::UpdateAutomation { actions, .. } = &operation.kind else {
            panic!("expected UpdateAutomation");
        };
        assert_eq!(
            actions,
            &[AutomationAction::SetPriority {
                priority: Priority::Elevated
            }]
        );

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("kind"), Some(&json!("update_automation")));
        assert_eq!(serialized, value);
    }

    #[test]
    fn delete_automation_round_trips() {
        let id = Uuid::new_v4();
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "delete_automation",
            "id": id
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::DeleteAutomation { id: parsed } = &operation.kind else {
            panic!("expected DeleteAutomation");
        };
        assert_eq!(*parsed, id);

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("kind"), Some(&json!("delete_automation")));
        assert_eq!(serialized, value);
    }

    #[test]
    fn create_event_round_trips() {
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "create_event",
            "id": Uuid::new_v4(),
            "project_id": Uuid::new_v4(),
            "title": "standup",
            "description": null,
            "interval": {
                "start": {"kind": "date", "date": "2026-11-16"},
                "end": {"kind": "date", "date": "2026-11-17"}
            },
            "created_at": "2026-01-01T00:00:00Z"
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::CreateEvent { interval, .. } = &operation.kind else {
            panic!("expected CreateEvent");
        };
        assert!(interval.is_all_day());

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("kind"), Some(&json!("create_event")));
        assert_eq!(serialized, value);
    }

    #[test]
    fn create_event_round_trips_recurrence() {
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "create_event",
            "id": Uuid::new_v4(),
            "project_id": Uuid::new_v4(),
            "title": "standup",
            "description": null,
            "interval": {
                "start": {"kind": "date", "date": "2026-11-16"},
                "end": {"kind": "date", "date": "2026-11-17"}
            },
            "recurrence": {
                "text": "FREQ=WEEKLY;BYDAY=MO",
                "exdates": [{"kind": "date", "date": "2026-11-23"}]
            },
            "created_at": "2026-01-01T00:00:00Z"
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::CreateEvent { recurrence, .. } = &operation.kind else {
            panic!("expected CreateEvent");
        };
        assert_eq!(
            recurrence,
            &Some(RRule {
                text: "FREQ=WEEKLY;BYDAY=MO".to_string(),
                exdates: vec![When::Date {
                    date: jiff::civil::date(2026, 11, 23)
                }],
            })
        );

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized, value);
    }

    #[test]
    fn update_event_distinguishes_absent_null_and_set_recurrence() {
        let operation_json = |recurrence: Option<serde_json::Value>| {
            let mut value = json!({
                "operation_id": Uuid::new_v4(),
                "operation_version": OPERATION_VERSION,
                "timestamp": null,
                "kind": "update_event",
                "id": Uuid::new_v4(),
            });
            if let Some(recurrence) = recurrence {
                value["recurrence"] = recurrence;
            }
            value
        };

        let absent: Operation = serde_json::from_value(operation_json(None)).unwrap();
        let OperationKind::UpdateEvent { recurrence, .. } = &absent.kind else {
            panic!("expected UpdateEvent");
        };
        assert_eq!(recurrence, &None);

        let cleared: Operation = serde_json::from_value(operation_json(Some(json!(null)))).unwrap();
        let OperationKind::UpdateEvent { recurrence, .. } = &cleared.kind else {
            panic!("expected UpdateEvent");
        };
        assert_eq!(recurrence, &Some(None));
        assert_eq!(
            serde_json::to_value(&cleared).unwrap().get("recurrence"),
            Some(&json!(null))
        );

        let rule = json!({ "text": "FREQ=DAILY" });
        let set: Operation = serde_json::from_value(operation_json(Some(rule.clone()))).unwrap();
        let OperationKind::UpdateEvent { recurrence, .. } = &set.kind else {
            panic!("expected UpdateEvent");
        };
        assert_eq!(
            recurrence,
            &Some(Some(RRule {
                text: "FREQ=DAILY".to_string(),
                exdates: Vec::new(),
            }))
        );
        assert_eq!(
            serde_json::to_value(&set).unwrap().get("recurrence"),
            Some(&rule)
        );
    }

    #[test]
    fn update_event_distinguishes_an_absent_description_from_a_clear() {
        let id = Uuid::new_v4();
        let base = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "update_event",
            "id": id,
            "title": "renamed"
        });
        let operation: Operation = serde_json::from_value(base).unwrap();
        let OperationKind::UpdateEvent { description, .. } = &operation.kind else {
            panic!("expected UpdateEvent");
        };
        assert!(description.is_none());

        let cleared = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "update_event",
            "id": id,
            "description": null
        });
        let operation: Operation = serde_json::from_value(cleared).unwrap();
        let OperationKind::UpdateEvent { description, .. } = &operation.kind else {
            panic!("expected UpdateEvent");
        };
        assert_eq!(description, &Some(None));
        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("description"), Some(&json!(null)));
    }

    #[test]
    fn delete_event_round_trips() {
        let id = Uuid::new_v4();
        let value = json!({
            "operation_id": Uuid::new_v4(),
            "operation_version": OPERATION_VERSION,
            "timestamp": null,
            "kind": "delete_event",
            "id": id
        });

        let operation: Operation = serde_json::from_value(value.clone()).unwrap();
        let OperationKind::DeleteEvent { id: parsed } = &operation.kind else {
            panic!("expected DeleteEvent");
        };
        assert_eq!(*parsed, id);

        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized.get("kind"), Some(&json!("delete_event")));
        assert_eq!(serialized, value);
    }
}
