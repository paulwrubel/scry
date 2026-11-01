//! Client-side access to the scry REST API.

mod commands;
pub use commands::{
    AddTask, MoveTask, UpdateTask, add, close_tasks, delete, duplicate, list, move_task, note_add,
    open_tasks, project_current, project_list, project_show, project_use, show, update,
};

mod http;
pub use http::Client;

mod models;
pub use models::{BatchTaskResult, ProjectDetail, TaskDetails};
