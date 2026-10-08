//! Project settings tools.

use rmcp::schemars;

use crate::error::AppError;
use scry_core::models::{Project, TaskSortingMode};
use crate::service::ProjectService;
use scry_core::store::Store;

use super::resolve_project;

/// Parameters for the `project_set_entry_status` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectSetEntryStatusParams {
    /// Status new tasks should default to; omit to clear it so new tasks use the first status.
    pub status: Option<String>,
    /// Project to configure; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn set_entry_status(
    store: &dyn Store,
    params: &ProjectSetEntryStatusParams,
) -> Result<Project, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    let status_id = match params.status.as_deref() {
        Some(name) => Some(service.get_status_by_name(&project, name).await?.id),
        None => None,
    };

    Ok(service
        .set_project_entry_status(&project, status_id)
        .await?)
}

/// Parameters for the `project_set_sort_mode` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectSetSortModeParams {
    /// Task ordering: alphabetical, alphabetical-case-insensitive, id, manual, or priority.
    #[schemars(with = "String")]
    pub mode: TaskSortingMode,
    /// Project to configure; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn set_sort_mode(
    store: &dyn Store,
    params: &ProjectSetSortModeParams,
) -> Result<Project, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service
        .set_project_sorting_mode(&project, params.mode)
        .await?)
}

/// Parameters for the `project_set_show_priority` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectSetShowPriorityParams {
    /// Whether to show task priority in listings.
    pub enabled: bool,
    /// Project to configure; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn set_show_priority(
    store: &dyn Store,
    params: &ProjectSetShowPriorityParams,
) -> Result<Project, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service
        .set_project_should_show_priority(&project, params.enabled)
        .await?)
}
