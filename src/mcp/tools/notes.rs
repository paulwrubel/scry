//! Note tools.

use rmcp::schemars;

use crate::error::AppError;
use crate::models::Note;
use crate::service::ProjectService;
use crate::store::sqlite::SqliteStore;

use super::resolve_project;

/// Parameters for the `note_add` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NoteAddParams {
    /// The task the note belongs to.
    pub task_id: i64,
    /// The note contents.
    pub contents: String,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn add_note(
    store: &SqliteStore,
    params: &NoteAddParams,
) -> Result<Note, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service
        .add_task_note(&project, params.task_id, params.contents.clone())
        .await?)
}
