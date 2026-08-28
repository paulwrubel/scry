use crate::tui::state::TaskWithNotes;
use crate::{
    models::{Note, Status, StatusID, Task, TaskID},
    tui::component::popup::ConfirmDeleteEntity,
};

/// Cross-cutting actions that components emit to the parent coordinator.
/// Internal component state changes (cursor movement, scrolling, text editing)
/// are handled within the component and return None from handle_event.
pub enum Action {
    // ── lifecycle ──
    Quit,

    // ── popup lifecycle ──
    OpenPopupAddNote(TaskID),
    OpenPopupAddOrEditTask(Option<TaskWithNotes>),
    OpenPopupConfirmDelete(ConfirmDeleteEntity),
    OpenPopupErrorInfo(String),
    DismissPopup,

    // ── commands ──
    CloseCommandInput,

    // ── tasks ──
    CreateTask(Task),
    UpdateTask(Task),
    DeleteTask(TaskID),

    // ── statuses ──
    CreateStatus(Status),
    UpdateStatus(Status),
    DeleteStatus(StatusID),

    // ── notes ──
    CreateNote(Note),
}
