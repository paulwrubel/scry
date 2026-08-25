use crate::{
    models::{Color, StatusID, TaskID},
    tui::component::popup::ConfirmDeleteEntity,
};

/// Cross-cutting actions that components emit to the parent coordinator.
/// Internal component state changes (cursor movement, scrolling, text editing)
/// are handled within the component and return None from handle_event.
pub enum Action {
    // ── lifecycle ──
    Quit,

    // ── input bar ──
    AddTask(String),

    // ── popup lifecycle ──
    OpenPopupConfirmDelete(ConfirmDeleteEntity),
    OpenPopupCreateTask,
    OpenPopupErrorInfo(String),
    DismissPopup,

    // ── commands ──
    CloseCommandInput,

    // ── task operations ──
    MoveTask {
        task_id: TaskID,
        status_id: StatusID,
    },
    DeleteTask(TaskID),

    // ── project settings ──
    #[allow(dead_code)]
    RenameProject(String),
    RenameStatus {
        id: StatusID,
        new_name: String,
    },
    AddStatus(String),
    DeleteStatus(StatusID),
    SetStatusColor {
        status_id: i64,
        color: Option<Color>,
    },
    ReorderStatus {
        id: StatusID,
        new_position: i32,
    },
}
