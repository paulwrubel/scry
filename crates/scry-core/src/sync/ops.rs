use crate::ids::{NoteId, ProjectId, SectionId, TaskId};
use crate::models::{Color, Lifecycle, Priority, Tags, TaskSortingMode};
use crate::position::Position;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// The version of the op wire format. Bump by one when the shape changes; the
/// server accepts the current version and one previous major version.
pub const OP_VERSION: u16 = 1;

/// Whether `version` is accepted by a peer whose current version is `current`.
/// Only the current and the immediately preceding version are accepted, so a
/// gap of two or more is rejected. The arithmetic avoids underflow at 0.
pub const fn op_version_supported_at(version: u16, current: u16) -> bool {
    version == current || (current > 1 && version == current - 1)
}

/// Whether this build accepts `version`.
pub fn op_version_supported(version: u16) -> bool {
    op_version_supported_at(version, OP_VERSION)
}

/// The lowest version this build accepts: one behind the current version, or
/// the current version when there is no previous one to accept.
pub fn min_supported_op_version() -> u16 {
    if OP_VERSION > 1 {
        OP_VERSION - 1
    } else {
        OP_VERSION
    }
}

/// A typed rejection of an op whose wire format this build cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    UnsupportedOpVersion {
        found: u16,
        current: u16,
        min_accepted: u16,
    },
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOpVersion {
                found,
                current,
                min_accepted,
            } => write!(
                f,
                "unsupported op version {found} (this server accepts {min_accepted}..={current})"
            ),
        }
    }
}

impl std::error::Error for OpError {}

/// One sync operation as it travels over the wire.
///
/// `timestamp` is an optional client-supplied wall-clock time that the server
/// records but never compares; REST and import ops leave it unset. `op_version`
/// is validated by [`Op::validate`] rather than during deserialization so one
/// unsupported op can still be acknowledged on its own instead of failing the
/// whole batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Op {
    pub op_id: Uuid,
    pub op_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub kind: OpKind,
}

impl Op {
    pub fn validate(&self) -> Result<(), OpError> {
        if op_version_supported(self.op_version) {
            Ok(())
        } else {
            Err(OpError::UnsupportedOpVersion {
                found: self.op_version,
                current: OP_VERSION,
                min_accepted: min_supported_op_version(),
            })
        }
    }
}

/// A section color update. `None` on [`OpKind::UpdateSection::color`] leaves the
/// color unchanged; [`ColorChange::Clear`] removes it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ColorChange {
    Clear,
    Set(Color),
}

/// The op vocabulary: one variant per `ProjectService` mutation, entity at a
/// time. Section merge is deliberately absent because it is a server-side rule
/// that emits ordinary ops. Moves and lifecycle changes stay separate so moving
/// a task never changes its lifecycle and closing or reopening it never changes
/// its section or position.
///
/// Create variants carry the client-generated id and `created_at` so
/// offline-created entities keep stable identity and Created ordering;
/// server-side REST creates fill those fields before building the op.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpKind {
    CreateProject {
        id: ProjectId,
        name: String,
        description: Option<String>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
        created_at: DateTime<Utc>,
    },
    /// `None` leaves a field unchanged; `Some("")` clears `description`.
    UpdateProject {
        id: ProjectId,
        name: Option<String>,
        description: Option<String>,
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
    /// `lifecycle` defaults to `Open` when absent.
    CreateTask {
        id: TaskId,
        project_id: ProjectId,
        title: String,
        description: Option<String>,
        priority: Priority,
        section_id: Option<SectionId>,
        position: Position,
        #[serde(default)]
        lifecycle: Lifecycle,
        tags: Tags,
        created_at: DateTime<Utc>,
    },
    /// Covers title, description, priority, and tags only: section and position
    /// belong to [`OpKind::MoveTask`], lifecycle to [`OpKind::SetLifecycle`].
    /// `None` leaves `description` unchanged and `Some("")` clears it.
    /// `Some(tags)` sets them, empty `Tags` clears them, `None` leaves them.
    UpdateTask {
        id: TaskId,
        title: Option<String>,
        description: Option<String>,
        priority: Option<Priority>,
        tags: Option<Tags>,
    },
    /// `section_id` and `position` are always set together; `None` means the
    /// task moves to No section.
    MoveTask {
        id: TaskId,
        section_id: Option<SectionId>,
        position: Position,
    },
    SetLifecycle {
        id: TaskId,
        lifecycle: Lifecycle,
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
