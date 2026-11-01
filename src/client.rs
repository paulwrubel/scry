//! Client-side access to the scry REST API.

mod commands;
pub use commands::{list, project_list};

mod http;
pub use http::Client;

mod models;
pub use models::ProjectDetail;
