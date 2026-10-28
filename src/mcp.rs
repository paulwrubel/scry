//! In-process Model Context Protocol (MCP) server exposing scry operations as tools.

use crate::client_state;
use crate::error::AppError;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use scry_core::store::ArcStore;
use serde::Serialize;

mod http;
mod tools;

pub use http::serve_http;

/// Build a caller-visible tool-level error result from an application error.
///
/// The payload mirrors the CLI's `--json` error document so the same `kind`
/// vocabulary appears on both surfaces.
pub(crate) fn error_result(error: impl Into<AppError>) -> CallToolResult {
    let error = error.into();
    let document = serde_json::json!({
        "error": {
            "kind": error.kind(),
            "message": error.to_string(),
        }
    });
    CallToolResult::error(vec![ContentBlock::text(document.to_string())])
}

/// Build a success result whose content is the compact JSON of `value`.
pub(crate) fn success_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value).map_err(|error| {
        McpError::internal_error(format!("failed to serialize tool result: {error}"), None)
    })?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Convert a store or service result into a tool result: `Ok` becomes JSON
/// content, `Err` becomes a caller-visible tool-level error.
pub(crate) fn tool_result<T, E>(result: Result<T, E>) -> Result<CallToolResult, McpError>
where
    T: Serialize,
    E: Into<AppError>,
{
    match result {
        Ok(value) => success_result(&value),
        Err(error) => Ok(error_result(error)),
    }
}

/// The scry MCP server. Holds the store that tool handlers operate on.
#[derive(Clone)]
pub struct ScryServer {
    store: ArcStore,
    tool_router: ToolRouter<ScryServer>,
}

#[tool_router]
impl ScryServer {
    pub fn new(store: ArcStore) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Report scry server information, including the active project")]
    async fn scry_info(&self) -> Result<CallToolResult, McpError> {
        let project = match client_state::active_project(self.store.as_ref()).await {
            Ok(project) => project,
            Err(error) => return Ok(error_result(error)),
        };
        success_result(&serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "active_project": project.name,
        }))
    }

    #[tool(description = "List all projects")]
    async fn project_list(&self) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::list_projects(self.store.as_ref()).await)
    }

    #[tool(description = "Show the active project")]
    async fn project_current(&self) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::active_project(self.store.as_ref()).await)
    }

    #[tool(description = "List the statuses of a project")]
    async fn status_list(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusListParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::list_statuses(self.store.as_ref(), &params).await)
    }

    #[tool(
        description = "List tasks in a project, optionally filtered by status and by title or tag search"
    )]
    async fn task_list(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskListParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::list_tasks(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Show full details for a task, including its notes")]
    async fn task_show(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskShowParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::show_task(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Add a task to a project")]
    async fn task_add(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::add_task(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Update a task's title, description, priority, tags, or status")]
    async fn task_update(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::update_task(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Move a task to a different status")]
    async fn task_move(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::move_task(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Duplicate a task into the same status")]
    async fn task_duplicate(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskDuplicateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::duplicate_task(self.store.as_ref(), &params).await)
    }

    #[tool(
        description = "Permanently delete a task",
        annotations(destructive_hint = true)
    )]
    async fn task_delete(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::delete_task(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Add a note to a task")]
    async fn note_add(
        &self,
        Parameters(params): Parameters<tools::notes::NoteAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::notes::add_note(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Create a project, optionally from a template (todolist or kanban)")]
    async fn project_create(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::create_project(self.store.as_ref(), &params).await)
    }

    #[tool(
        description = "Update a project's name, description, entry status, sort mode, or priority visibility"
    )]
    async fn project_update(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::update_project(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Set the active project")]
    async fn project_use(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectUseParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::use_project(self.store.as_ref(), &params).await)
    }

    #[tool(
        description = "Permanently delete a project and all of its tasks",
        annotations(destructive_hint = true)
    )]
    async fn project_delete(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::delete_project(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Add a status to a project")]
    async fn status_add(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::add_status(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Rename a status")]
    async fn status_rename(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusRenameParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::rename_status(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Move a status up or down within a project's ordering")]
    async fn status_move(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::move_status(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Set or clear a status's color")]
    async fn status_set_color(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusSetColorParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::set_status_color(self.store.as_ref(), &params).await)
    }

    #[tool(description = "Set a status's style")]
    async fn status_set_style(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusSetStyleParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::set_status_style(self.store.as_ref(), &params).await)
    }

    #[tool(
        description = "Remove an empty status from a project",
        annotations(destructive_hint = true)
    )]
    async fn status_remove(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusRemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::remove_status(self.store.as_ref(), &params).await)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ScryServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("scry", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "scry is a task manager. Use these tools to manage tasks, projects, statuses, and notes.",
            )
    }
}

/// Serve the MCP protocol over stdio until the client disconnects.
pub async fn serve_stdio(store: ArcStore) -> Result<(), AppError> {
    let server = ScryServer::new(store);
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|error| AppError::Internal(format!("failed to start MCP server: {error}")))?;
    service
        .waiting()
        .await
        .map_err(|error| AppError::Internal(format!("MCP server error: {error}")))?;
    Ok(())
}
