//! REST API handlers. Each resource lives in its own submodule; this module
//! aggregates them and holds the pieces shared across resources.

use crate::error::AppError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

mod close_task;
pub use close_task::close_task;

mod create_note;
pub use create_note::create_note;

mod create_project;
pub use create_project::create_project;

mod create_section;
pub use create_section::create_section;

mod create_task;
pub use create_task::create_task;

mod delete_note;
pub use delete_note::delete_note;

mod delete_project_by_id;
pub use delete_project_by_id::delete_project_by_id;

mod delete_section;
pub use delete_section::delete_section;

mod delete_task;
pub use delete_task::delete_task;

mod get_note;
pub use get_note::get_note;

mod get_notes_by_project;
pub use get_notes_by_project::get_notes_by_project;

mod get_notes_by_task;
pub use get_notes_by_task::get_notes_by_task;

mod get_project;
pub use get_project::get_project;

mod get_project_by_name;
pub use get_project_by_name::get_project_by_name;

mod get_project_snapshot;
pub use get_project_snapshot::get_project_snapshot;

mod get_projects;
pub use get_projects::get_projects;

mod get_section;
pub use get_section::get_section;

mod get_section_by_name;
pub use get_section_by_name::get_section_by_name;

mod get_sections;
pub use get_sections::get_sections;

mod get_task;
pub use get_task::get_task;

mod get_tasks_by_project;
pub use get_tasks_by_project::get_tasks_by_project;

mod get_tasks_by_section;
pub use get_tasks_by_section::get_tasks_by_section;

mod import;
pub use import::import;

mod merge_section;
pub use merge_section::merge_section;

mod move_section;
pub use move_section::move_section;

mod move_task;
pub use move_task::move_task;

mod open_task;
pub use open_task::open_task;

mod update_note;
pub use update_note::update_note;

mod update_project;
pub use update_project::update_project;

mod update_section;
pub use update_section::update_section;

mod update_task;
pub use update_task::update_task;

/// JSON body returned for every failed request.
#[derive(Serialize)]
pub struct ApiErrorJson {
    pub code: u16,
    pub kind: String,
    pub message: String,
}

impl From<AppError> for Response {
    fn from(error: AppError) -> Self {
        let status = match error.kind() {
            "not_found" => StatusCode::NOT_FOUND,
            "invalid" => StatusCode::BAD_REQUEST,
            "conflict" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = ApiErrorJson {
            code: status.as_u16(),
            kind: error.kind().to_string(),
            message: error.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

/// Server metadata returned by the index endpoint.
#[derive(Serialize)]
pub struct ServerInfo {
    pub name: &'static str,
    pub version: &'static str,
}

/// Report the server name and version.
pub async fn index() -> Json<ServerInfo> {
    Json(ServerInfo {
        name: "scry",
        version: env!("CARGO_PKG_VERSION"),
    })
}
