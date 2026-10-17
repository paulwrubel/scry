//! REST API handlers. Each resource lives in its own submodule; this module
//! aggregates them and holds the pieces shared across resources.

use crate::error::AppError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

mod create_note;
pub use create_note::create_note;

mod create_project;
pub use create_project::create_project;

mod create_status;
pub use create_status::create_status;

mod create_task;
pub use create_task::create_task;

mod delete_note;
pub use delete_note::delete_note;

mod delete_project;
pub use delete_project::delete_project;

mod delete_project_by_id;
pub use delete_project_by_id::delete_project_by_id;

mod delete_status;
pub use delete_status::delete_status;

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

mod get_status;
pub use get_status::get_status;

mod get_status_by_name;
pub use get_status_by_name::get_status_by_name;

mod get_statuses;
pub use get_statuses::get_statuses;

mod get_task;
pub use get_task::get_task;

mod get_tasks_by_project;
pub use get_tasks_by_project::get_tasks_by_project;

mod get_tasks_by_status;
pub use get_tasks_by_status::get_tasks_by_status;

mod import;
pub use import::import;

mod move_status;
pub use move_status::move_status;

mod move_task;
pub use move_task::move_task;

mod update_note;
pub use update_note::update_note;

mod update_project;
pub use update_project::update_project;

mod update_status;
pub use update_status::update_status;

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
