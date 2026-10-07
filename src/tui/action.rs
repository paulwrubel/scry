use crate::models::{Project, Status, StatusId, TaskId};
use crate::service::TaskInput;
use crate::state::TaskWithNotes;
use crate::tui::component::popup::ConfirmDeleteEntity;

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
    UpdateTask { id: TaskId, input: TaskInput },
    DuplicateTask(TaskId),
    DeleteTask(TaskId),

    // ── notes ──
    AddTaskNote { task_id: TaskId, contents: String },

    // ── statuses ──
    CreateStatus(Status),
    UpdateStatus(Status),
    DeleteStatus(StatusId),

    // ── projects ──
    UpdateProject(Project),
}
