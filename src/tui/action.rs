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
