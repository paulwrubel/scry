//! Project tools.

use rmcp::schemars;

use crate::error::AppError;
use crate::models::{PROJECT_TEMPLATES, Project};
use crate::service::ProjectService;
use crate::store::Store;

use super::resolve_project;

pub(crate) async fn list_projects(store: &dyn Store) -> Result<Vec<Project>, AppError> {
    Ok(store.get_all_projects().await?)
}

pub(crate) async fn active_project(store: &dyn Store) -> Result<Project, AppError> {
    Ok(store.get_active_project().await?)
}

/// Parameters for the `project_create` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectCreateParams {
    /// The new project name.
    pub name: String,
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
        .create_project(params.name.clone(), template)
        .await?)
}

/// Parameters for the `project_rename` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectRenameParams {
    /// The current project name.
    pub old_name: String,
    /// The new project name.
    pub new_name: String,
}

pub(crate) async fn rename_project(
    store: &dyn Store,
    params: &ProjectRenameParams,
) -> Result<Project, AppError> {
    let project = resolve_project(store, Some(params.old_name.as_str())).await?;
    let service = ProjectService::new(store);

    Ok(service
        .rename_project(&project, params.new_name.clone())
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
    params: &ProjectUseParams,
) -> Result<Project, AppError> {
    let service = ProjectService::new(store);

    Ok(service.set_active_project(&params.name).await?)
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
