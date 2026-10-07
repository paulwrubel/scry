//! Task tools.

use rmcp::schemars;

use crate::error::{AppError, ServiceError};
use crate::models::Task;
use crate::state::{ProjectState, TaskWithNotes};
use crate::store::sqlite::SqliteStore;

use super::resolve_project;

/// Parameters for the `task_list` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskListParams {
    /// Only list tasks in this status.
    pub status: Option<String>,
    /// Case-insensitive substring match over task titles and tags.
    pub search: Option<String>,
    /// Project to inspect; defaults to the active project.
    pub project: Option<String>,
}

/// Parameters for the `task_show` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskShowParams {
    /// The task id.
    pub id: i64,
    /// Project to inspect; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn list_tasks(
    store: &SqliteStore,
    params: &TaskListParams,
) -> Result<Vec<Task>, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let mut state = ProjectState::load_from_store(store, project.id).await?;

    if let Some(search) = params.search.as_deref().filter(|search| !search.is_empty()) {
        state = state.with_substring_filter(search.to_string());
    }

    Ok(state
        .statuses()
        .filter(|status| match &params.status {
            Some(filter) => status.name == *filter,
            None => true,
        })
        .flat_map(|status| state.tasks_in_status(status.id))
        .map(Task::from)
        .collect())
}

pub(crate) async fn show_task(
    store: &SqliteStore,
    params: &TaskShowParams,
) -> Result<TaskWithNotes, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let state = ProjectState::load_from_store(store, project.id).await?;

    state.get_task_by_id(params.id).cloned().ok_or_else(|| {
        AppError::Service(ServiceError::TaskNotFound {
            task_id: params.id,
            project: project.name.clone(),
        })
    })
}
