use crate::ids::{NoteId, ProjectId, SectionId, TaskId};
use crate::models::{Color, Priority, Tags, TaskSortingMode, TaskState};
use crate::position::Position;
use chrono::{DateTime, Utc};
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
/// `timestamp` is an optional client-supplied wall-clock time that the server
/// records but never compares; REST and import operations leave it unset. `operation_version`
/// is validated by [`Operation::validate`] rather than during deserialization so one
/// unsupported operation can still be acknowledged on its own instead of failing the
/// whole batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Operation {
    pub operation_id: Uuid,
    pub operation_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationKind {
    CreateProject {
        id: ProjectId,
        name: String,
        description: Option<String>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
        created_at: DateTime<Utc>,
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
        task_sorting_mode: Option<TaskSortingMode>,
        show_priority: Option<bool>,
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
        created_at: DateTime<Utc>,
    },
    UpdateSection {
        id: SectionId,
        name: Option<String>,
        color: Option<ColorChange>,
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
        tags: Tags,
        created_at: DateTime<Utc>,
    },
    /// Covers title, description, priority, tags, and state: section and position
    /// belong to [`OperationKind::MoveTask`].
    /// `description` distinguishes all three states: `None` leaves it,
    /// `Some(None)` clears it, `Some(Some(text))` sets it.
    /// `Some(tags)` sets them, empty `Tags` clears them, `None` leaves them.
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
    },
    /// `section_id` and `position` are always set together; `None` means the
    /// task moves to No section.
    MoveTask {
        id: TaskId,
        section_id: Option<SectionId>,
        position: Position,
    },
    DeleteTask {
        id: TaskId,
    },
    CreateNote {
        id: NoteId,
        task_id: TaskId,
        contents: String,
        created_at: DateTime<Utc>,
    },
    UpdateNote {
        id: NoteId,
        contents: String,
    },
    DeleteNote {
        id: NoteId,
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
}
