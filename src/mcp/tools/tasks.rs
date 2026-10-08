//! Task tools.

use rmcp::schemars;

use crate::error::{AppError, ServiceError};
use crate::models::{Priority, Tags, Task};
use crate::service::{ProjectService, TaskInput};
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

/// Parameters for the `task_show` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskShowParams {
    /// The task id.
    pub id: i64,
    /// Project to inspect; defaults to the active project.
    pub project: Option<String>,
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

/// Parameters for the `task_add` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskAddParams {
    /// The task title.
    pub title: String,
    /// The task description.
    pub description: Option<String>,
    /// Task priority: minimal, low, medium, high, or critical.
    #[schemars(with = "Option<String>")]
    pub priority: Option<Priority>,
    /// Tags to apply to the task.
    pub tags: Option<Vec<String>>,
    /// Target status name; defaults to the project's entry status.
    pub status: Option<String>,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn add_task(
    store: &SqliteStore,
    params: &TaskAddParams,
) -> Result<Task, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    let status = match params.status.as_deref() {
        Some(name) => Some(service.get_status_by_name(&project, name).await?.id),
        None => None,
    };

    let input = TaskInput {
        title: Some(params.title.clone()),
        description: params.description.clone(),
        priority: params.priority,
        tags: params.tags.clone().map(Tags::from),
        status,
    };

    Ok(service.create_task(&project, input).await?.task)
}

/// Parameters for the `task_update` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskUpdateParams {
    /// The task id.
    pub id: i64,
    /// New title.
    pub title: Option<String>,
    /// New description; an empty string clears it.
    pub description: Option<String>,
    /// New priority: minimal, low, medium, high, or critical.
    #[schemars(with = "Option<String>")]
    pub priority: Option<Priority>,
    /// Replacement tags; an empty list clears them.
    pub tags: Option<Vec<String>>,
    /// Move the task to this status.
    pub status: Option<String>,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn update_task(
    store: &SqliteStore,
    params: &TaskUpdateParams,
) -> Result<Task, AppError> {
    if params.title.is_none()
        && params.description.is_none()
        && params.priority.is_none()
        && params.tags.is_none()
        && params.status.is_none()
    {
        return Err(AppError::Usage(
            "no task fields provided to update".to_string(),
        ));
    }

    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    let status = match params.status.as_deref() {
        Some(name) => Some(service.get_status_by_name(&project, name).await?.id),
        None => None,
    };

    let patch = TaskInput {
        title: params.title.clone(),
        description: params.description.clone(),
        priority: params.priority,
        tags: params.tags.clone().map(Tags::from),
        status,
    };

    Ok(service.update_task(&project, params.id, patch).await?)
}

/// Parameters for the `task_move` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskMoveParams {
    /// The task id.
    pub id: i64,
    /// The target status name.
    pub status: String,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn move_task(
    store: &SqliteStore,
    params: &TaskMoveParams,
) -> Result<Task, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service.get_status_by_name(&project, &params.status).await?;

    Ok(service
        .move_task(&project, params.id, status.id)
        .await?
        .task)
}

/// Parameters for the `task_duplicate` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskDuplicateParams {
    /// The task id.
    pub id: i64,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn duplicate_task(
    store: &SqliteStore,
    params: &TaskDuplicateParams,
) -> Result<Task, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service.duplicate_task(&project, params.id).await?.task)
}

/// Parameters for the `task_delete` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskDeleteParams {
    /// The task id.
    pub id: i64,
    /// Project to operate on; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn delete_task(
    store: &SqliteStore,
    params: &TaskDeleteParams,
) -> Result<Task, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service.delete_task(&project, params.id).await?)
}
