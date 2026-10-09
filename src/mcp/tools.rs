//! Tool implementations, grouped by the resource they operate on.

use crate::error::AppError;
use scry_core::error::StorageError;
use scry_core::models::Project;
use scry_core::store::Store;

pub(crate) mod notes;
pub(crate) mod project_settings;
pub(crate) mod projects;
pub(crate) mod statuses;
pub(crate) mod tasks;

/// Resolve which project a tool should act on: an explicit name when given,
/// otherwise the active project. A missing name is reported as a not-found
/// tool error so the agent can correct itself.
pub(crate) async fn resolve_project(
    store: &dyn Store,
    name: Option<&str>,
) -> Result<Project, AppError> {
    match name {
        Some(name) => store.get_project_by_name(name).await?.ok_or_else(|| {
            AppError::Storage(StorageError::NotFound(format!(
                "project '{name}' not found"
            )))
        }),
        None => Ok(store.get_active_project().await?),
    }
}
