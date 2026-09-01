use crate::models::Project;
use crate::store::TaskToCreate;
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

    OpenPopupAddNote(TaskID),
    OpenPopupAddOrEditTask(Option<TaskWithNotes>),
    OpenPopupConfirmDelete(ConfirmDeleteEntity),
    OpenPopupErrorInfo(String),
    DismissPopup,

    CloseCommandInput,
    CloseFilterInput,

    // ── tasks ──
    CreateTask(TaskToCreate),
    UpdateTask(Task),
    DeleteTask(TaskID),

    // ── notes ──
    CreateNote(Note),

    // ── statuses ──
    CreateStatus(Status),
    UpdateStatus(Status),
    DeleteStatus(StatusID),

    // ── projects ──
    UpdateProject(Project),
}
