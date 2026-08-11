/// Cross-cutting actions that components emit to the parent coordinator.
/// Internal component state changes (cursor movement, scrolling, text editing)
/// are handled within the component and return None from handle_event.
pub enum Action {
    // ── lifecycle ──
    Quit,

    // ── input bar ──
    FocusInput,
    AddTask(String),

    // ── task operations ──
    OpenPopupTaskDetail(i64),
    OpenPopupMovePicker(i64),
    OpenPopupDeleteConfirm(i64),
    MoveTask {
        task_id: i64,
        status_name: String,
    },
    DeleteTask(i64),

    // ── focus movement ──
    MoveFocusDown,
    MoveFocusUp,

    // ── popup lifecycle ──
    DismissPopup,

    // ── project settings ──
    #[allow(dead_code)]
    RenameProject(String),
    #[allow(dead_code)]
    RenameStatus {
        old_name: String,
        new_name: String,
    },
    #[allow(dead_code)]
    AddStatus(String),
    #[allow(dead_code)]
    DeleteStatus(String),
    #[allow(dead_code)]
    SetStatusColor {
        status_id: i64,
        color: Option<String>,
    },
    #[allow(dead_code)]
    ReorderStatus {
        status_name: String,
        new_position: i32,
    },
}
