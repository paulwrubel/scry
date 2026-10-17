use crate::service::TaskInput;
use crate::state::TaskWithNotes;
use crate::tui::component::popup::ConfirmDeleteEntity;
use scry_core::models::{Color, StatusId, StatusStyle, TaskId, TaskSortingMode};

/// Cross-cutting actions that components emit to the parent coordinator.
/// Internal component state changes (cursor movement, scrolling, text editing)
/// are handled within the component and return None from handle_event.
pub enum Action {
    // ── lifecycle ──
    Quit,

    /// Force a reload of the cached project snapshot, picking up edits made
    /// by other clients (CLI, another TUI, an agent over MCP/HTTP).
    Refresh,

    OpenPopupAddNote(TaskId),
    OpenPopupAddOrEditTask(Option<TaskWithNotes>),
    OpenPopupConfirmDelete(ConfirmDeleteEntity),
    OpenPopupErrorInfo(String),
    DismissPopup,

    CloseCommandInput,
    CloseFilterInput,

    // ── tasks ──
    CreateTask(TaskInput),
    UpdateTask {
        id: TaskId,
        input: TaskInput,
    },
    DuplicateTask(TaskId),
    DeleteTask(TaskId),

    // ── notes ──
    AddTaskNote {
        task_id: TaskId,
        contents: String,
    },

    // ── statuses ──
    CreateStatus {
        name: String,
    },
    RenameStatus {
        status_id: StatusId,
        new_name: String,
    },
    SetStatusColor {
        status_id: StatusId,
        color: Option<Color>,
    },
    SetStatusStyle {
        status_id: StatusId,
        style: StatusStyle,
    },
    MoveStatusUp {
        status_id: StatusId,
    },
    MoveStatusDown {
        status_id: StatusId,
    },
    DeleteStatus {
        status_id: StatusId,
    },

    // ── projects ──
    SetProjectEntryStatus {
        status_id: Option<StatusId>,
    },
    SetProjectSortingMode(TaskSortingMode),
    SetProjectShouldShowPriority(bool),
}

impl Action {
    /// Actions that invalidate the cached project snapshot: every action that
    /// attempts a store write, plus an explicit refresh.
    pub fn is_mutable(&self) -> bool {
        matches!(
            self,
            Action::CreateTask(_)
                | Action::UpdateTask { .. }
                | Action::DuplicateTask(_)
                | Action::DeleteTask(_)
                | Action::AddTaskNote { .. }
                | Action::CreateStatus { .. }
                | Action::RenameStatus { .. }
                | Action::SetStatusColor { .. }
                | Action::SetStatusStyle { .. }
                | Action::MoveStatusUp { .. }
                | Action::MoveStatusDown { .. }
                | Action::DeleteStatus { .. }
                | Action::SetProjectEntryStatus { .. }
                | Action::SetProjectSortingMode(_)
                | Action::SetProjectShouldShowPriority(_)
                | Action::Refresh
        )
    }
}
