//! Client-side access to the scry REST API.

mod commands;
pub use commands::{
    AddTask, AutomationActionChoice, AutomationTriggerKind, CreateProject, ListOptions, MoveTask,
    Provider, SectionMovePlacement, TaskMovePlacement, UpdateProject, UpdateTask, add,
    automation_list, automation_remove, automation_set, close_tasks, comment_add, delete,
    duplicate, event_add, event_delete, event_list, event_show, event_update, export, import, list,
    move_task, open_tasks, project_create, project_current, project_delete, project_list,
    project_show, project_update, project_use, section_add, section_list, section_merge,
    section_move, section_remove, section_rename, section_reset_color, section_set_color, show,
    sync, update,
};

mod http;
pub use http::Client;

mod models;
pub use models::{BatchTaskResult, ProjectDetail, TaskDetails};
