//! Project and status tools.

use rmcp::schemars;

use crate::error::AppError;
use crate::models::{Project, Status};
use crate::store::{TaskStore, sqlite::SqliteStore};

use super::resolve_project;

/// Parameters for the `status_list` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusListParams {
    /// Project to inspect; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn list_projects(store: &SqliteStore) -> Result<Vec<Project>, AppError> {
    Ok(store.get_all_projects().await?)
}

pub(crate) async fn active_project(store: &SqliteStore) -> Result<Project, AppError> {
    Ok(store.get_active_project().await?)
}

pub(crate) async fn list_statuses(
    store: &SqliteStore,
    project: Option<&str>,
) -> Result<Vec<Status>, AppError> {
    let project = resolve_project(store, project).await?;
    Ok(store.get_all_statuses_by_project_id(project.id).await?)
}
