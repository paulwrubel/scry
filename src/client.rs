//! Client-side access to the scry REST API.

mod commands;
pub use commands::{
    AddTask, CreateProject, MoveTask, UpdateProject, UpdateTask, add, close_tasks, delete,
    duplicate, list, move_task, note_add, open_tasks, project_create, project_current,
    project_delete, project_list, project_show, project_update, project_use, show, update,
};

mod http;
pub use http::Client;

mod models;
pub use models::{BatchTaskResult, ProjectDetail, TaskDetails};
