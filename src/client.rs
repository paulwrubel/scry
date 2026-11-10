//! Client-side access to the scry REST API.

mod commands;
pub use commands::{
    AddTask, AutomationActionChoice, AutomationTriggerKind, CreateProject, MoveTask,
    SectionMovePlacement, TaskMovePlacement, UpdateProject, UpdateTask, add, automation_list,
    automation_remove, automation_set, close_tasks, delete, duplicate, export, import, list,
    move_task, note_add, open_tasks, project_create, project_current, project_delete, project_list,
    project_show, project_update, project_use, section_add, section_list, section_merge,
    section_move, section_remove, section_rename, section_reset_color, section_set_color, show,
    update,
};

mod http;
pub use http::Client;

mod models;
pub use models::{BatchTaskResult, ProjectDetail, TaskDetails};
