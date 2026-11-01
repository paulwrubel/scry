//! Client-side access to the scry REST API.

mod commands;
pub use commands::{
    AddTask, MoveTask, UpdateTask, add, list, move_task, project_list, show, update,
};

mod http;
pub use http::Client;

mod models;
pub use models::{ProjectDetail, TaskDetails};
