//! Project tools.

use super::resolve_project;
use crate::client_state;
use crate::config::ScryConfig;
use crate::error::AppError;
use crate::service::{EntryStatusUpdate, ProjectService, ProjectUpdate};
use rmcp::schemars;
use scry_core::models::{PROJECT_TEMPLATES, Project, TaskSortingMode};
use scry_core::store::Store;

pub(crate) async fn list_projects(store: &dyn Store) -> Result<Vec<Project>, AppError> {
    Ok(store.get_all_projects().await?)
}

pub(crate) async fn active_project(
    store: &dyn Store,
    config: &ScryConfig,
) -> Result<Project, AppError> {
    client_state::active_project(store, config).await
}

/// Parameters for the `project_create` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectCreateParams {
    /// The new project name.
    pub name: String,
    /// Optional freeform project description.
    pub description: Option<String>,
    /// Optional template seeding statuses and settings: `todolist` or `kanban`.
    pub template: Option<String>,
}

pub(crate) async fn create_project(
    store: &dyn Store,
    params: &ProjectCreateParams,
) -> Result<Project, AppError> {
    let service = ProjectService::new(store);

    let template = match params.template.as_deref() {
        Some(requested) => Some(
            *PROJECT_TEMPLATES
                .iter()
                .find(|template| template.name == requested)
                .ok_or_else(|| {
                    AppError::Usage(format!("unknown template name: \"{requested}\""))
                })?,
        ),
        None => None,
    };

    Ok(service
        .create_project(params.name.clone(), params.description.clone(), template)
        .await?)
}

/// Parameters for the `project_update` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectUpdateParams {
    /// Project to update; defaults to the active project.
    pub project: Option<String>,
    /// New project name.
    pub name: Option<String>,
    /// New description; an empty string clears it.
    pub description: Option<String>,
    /// Status new tasks should default to.
    pub entry_status: Option<String>,
    /// Clear the entry status so new tasks use the first status.
    pub clear_entry_status: Option<bool>,
    /// Task ordering: alphabetical, alphabetical-case-insensitive, id, manual, or priority.
    #[schemars(with = "Option<String>")]
    pub sort_mode: Option<TaskSortingMode>,
    /// Show or hide task priority in listings.
    pub show_priority: Option<bool>,
}

pub(crate) async fn update_project(
    store: &dyn Store,
    config: &ScryConfig,
    params: &ProjectUpdateParams,
) -> Result<Project, AppError> {
    let clear_entry_status = params.clear_entry_status.unwrap_or(false);
    if params.name.is_none()
        && params.description.is_none()
        && params.entry_status.is_none()
        && params.sort_mode.is_none()
        && !clear_entry_status
        && params.show_priority.is_none()
    {
        return Err(AppError::Usage(
            "no project fields provided to update".to_string(),
        ));
    }
    if params.entry_status.is_some() && clear_entry_status {
        return Err(AppError::Usage(
            "cannot set and clear the entry status at the same time".to_string(),
        ));
    }

    let project = resolve_project(store, config, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    let entry_status = if clear_entry_status {
        EntryStatusUpdate::Clear
    } else if let Some(name) = params.entry_status.as_deref() {
        let status = service.get_status_by_name(&project, name).await?;
        EntryStatusUpdate::Set(status.id)
    } else {
        EntryStatusUpdate::Unchanged
    };

    Ok(service
        .update_project(
            &project,
            ProjectUpdate {
                name: params.name.clone(),
                description: params.description.clone(),
                entry_status,
                task_sorting_mode: params.sort_mode,
                show_priority: params.show_priority,
            },
        )
        .await?)
}

/// Parameters for the `project_use` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectUseParams {
    /// The project to make active.
    pub name: String,
}

pub(crate) async fn use_project(
    store: &dyn Store,
    config: &ScryConfig,
    params: &ProjectUseParams,
) -> Result<Project, AppError> {
    client_state::set_active_project(store, config, &params.name).await
}

/// Parameters for the `project_delete` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectDeleteParams {
    /// The project to delete.
    pub name: String,
}

pub(crate) async fn delete_project(
    store: &dyn Store,
    params: &ProjectDeleteParams,
) -> Result<Project, AppError> {
    let service = ProjectService::new(store);

    Ok(service.delete_project(&params.name).await?)
}
