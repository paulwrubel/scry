//! Status tools.

use rmcp::schemars;

use crate::error::AppError;
use crate::models::{Color, Status, StatusStyle};
use crate::service::ProjectService;
use crate::store::Store;

use super::resolve_project;

/// Parameters for the `status_list` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusListParams {
    /// Project to inspect; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn list_statuses(
    store: &dyn Store,
    params: &StatusListParams,
) -> Result<Vec<Status>, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    Ok(store.get_all_statuses_by_project_id(project.id).await?)
}

/// Parameters for the `status_add` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusAddParams {
    /// Name of the new status.
    pub name: String,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn add_status(
    store: &dyn Store,
    params: &StatusAddParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);

    Ok(service.create_status(&project, params.name.clone()).await?)
}

/// Parameters for the `status_remove` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusRemoveParams {
    /// Name of the status to remove. The status must have no tasks.
    pub name: String,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn remove_status(
    store: &dyn Store,
    params: &StatusRemoveParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service.get_status_by_name(&project, &params.name).await?;

    Ok(service.delete_status(&project, status.id).await?)
}

/// Parameters for the `status_rename` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusRenameParams {
    /// Current name of the status.
    pub old_name: String,
    /// New name for the status.
    pub new_name: String,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn rename_status(
    store: &dyn Store,
    params: &StatusRenameParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service
        .get_status_by_name(&project, &params.old_name)
        .await?;

    Ok(service
        .rename_status(&project, status.id, params.new_name.clone())
        .await?)
}

/// Direction to move a status within its project's ordering.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StatusMoveDirection {
    /// Toward the start of the ordering.
    Up,
    /// Toward the end of the ordering.
    Down,
}

/// Parameters for the `status_move` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusMoveParams {
    /// Name of the status to move.
    pub name: String,
    /// Which way to move it.
    pub direction: StatusMoveDirection,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn move_status(
    store: &dyn Store,
    params: &StatusMoveParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service.get_status_by_name(&project, &params.name).await?;

    let moved = match params.direction {
        StatusMoveDirection::Up => service.move_status_up(&project, status.id).await?,
        StatusMoveDirection::Down => service.move_status_down(&project, status.id).await?,
    };

    Ok(moved.unwrap_or(status))
}

/// Parameters for the `status_set_color` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusSetColorParams {
    /// Name of the status to recolor.
    pub name: String,
    /// Color to apply (black, red, green, yellow, blue, magenta, cyan, gray, dark-gray,
    /// light-red, light-green, light-yellow, light-blue, light-magenta, light-cyan, white).
    /// Omit to clear the color.
    #[schemars(with = "Option<String>")]
    pub color: Option<Color>,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn set_status_color(
    store: &dyn Store,
    params: &StatusSetColorParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service.get_status_by_name(&project, &params.name).await?;

    Ok(service
        .set_status_color(&project, status.id, params.color)
        .await?)
}

/// Parameters for the `status_set_style` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StatusSetStyleParams {
    /// Name of the status to restyle.
    pub name: String,
    /// Style to apply: none, unchecked, checked, strikethrough, or hidden.
    #[schemars(with = "String")]
    pub style: StatusStyle,
    /// Project to modify; defaults to the active project.
    pub project: Option<String>,
}

pub(crate) async fn set_status_style(
    store: &dyn Store,
    params: &StatusSetStyleParams,
) -> Result<Status, AppError> {
    let project = resolve_project(store, params.project.as_deref()).await?;
    let service = ProjectService::new(store);
    let status = service.get_status_by_name(&project, &params.name).await?;

    Ok(service
        .set_status_style(&project, status.id, params.style)
        .await?)
}
