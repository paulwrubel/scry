//! Client-side access to the scry REST API.

mod commands;
pub use commands::{AddTask, add, list, project_list, show};

mod http;
pub use http::Client;

mod models;
pub use models::{ProjectDetail, TaskDetails};
