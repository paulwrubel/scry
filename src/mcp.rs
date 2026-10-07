//! In-process Model Context Protocol (MCP) server exposing scry operations as tools.

use crate::error::{AppError, StorageError};
use crate::models::Project;
use crate::store::TaskStore;
use crate::store::sqlite::SqliteStore;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

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

/// Resolve which project a tool should act on: an explicit name when given,
/// otherwise the active project. A missing name is reported as a not-found
/// tool error so the agent can correct itself.
pub(crate) async fn resolve_project(
    store: &SqliteStore,
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

/// Parameters for the `status_list` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StatusListParams {
    /// Project to inspect; defaults to the active project.
    project: Option<String>,
}

/// The scry MCP server. Holds the store that tool handlers operate on.
#[derive(Clone)]
pub struct ScryServer {
    store: SqliteStore,
    tool_router: ToolRouter<ScryServer>,
}

#[tool_router]
impl ScryServer {
    pub fn new(store: SqliteStore) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Report scry server information, including the active project")]
    async fn scry_info(&self) -> Result<CallToolResult, McpError> {
        let project = match self.store.get_active_project().await {
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
        tool_result(self.store.get_all_projects().await)
    }

    #[tool(description = "Show the active project")]
    async fn project_current(&self) -> Result<CallToolResult, McpError> {
        tool_result(self.store.get_active_project().await)
    }

    #[tool(description = "List the statuses of a project")]
    async fn status_list(
        &self,
        Parameters(params): Parameters<StatusListParams>,
    ) -> Result<CallToolResult, McpError> {
        let project = match resolve_project(&self.store, params.project.as_deref()).await {
            Ok(project) => project,
            Err(error) => return Ok(error_result(error)),
        };
        tool_result(self.store.get_all_statuses_by_project_id(project.id).await)
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
pub async fn serve_stdio(store: SqliteStore) -> Result<(), AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    async fn test_server() -> (TempDir, ScryServer) {
        let dir = TempDir::new().expect("temp dir");
        let url = format!("sqlite://{}", dir.path().join("scry.db").display());
        let store = SqliteStore::new(&url).await.expect("store");
        (dir, ScryServer::new(store))
    }

    fn text_of(result: &CallToolResult) -> String {
        let value = serde_json::to_value(result).expect("serialize result");
        value["content"][0]["text"]
            .as_str()
            .expect("text content block")
            .to_string()
    }

    #[tokio::test]
    async fn scry_info_reports_version_and_active_project() {
        let dir = TempDir::new().expect("temp dir");
        let url = format!("sqlite://{}", dir.path().join("scry.db").display());
        let store = SqliteStore::new(&url).await.expect("store");
        let server = ScryServer::new(store);

        let result = server.scry_info().await.expect("scry_info");
        let json = serde_json::to_string(&result).expect("serialize result");

        assert!(
            json.contains(env!("CARGO_PKG_VERSION")),
            "version missing from result: {json}"
        );
        assert!(
            json.contains("default"),
            "active project missing from result: {json}"
        );
    }

    #[test]
    fn error_result_carries_the_stable_kind_and_message() {
        let error = AppError::Service(crate::error::ServiceError::TaskNotFound {
            task_id: 7,
            project: "default".to_string(),
        });

        let result = error_result(error);

        assert_eq!(result.is_error, Some(true), "expected a tool-level error");
        let value = serde_json::to_value(&result).expect("serialize result");
        let text = value["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert!(
            text.contains("\"kind\":\"not_found\""),
            "kind missing: {text}"
        );
        assert!(text.contains("Task 7 not found"), "message missing: {text}");
    }

    #[tokio::test]
    async fn project_list_returns_projects() {
        let (_dir, server) = test_server().await;
        let result = server.project_list().await.expect("project_list");
        assert!(
            text_of(&result).contains("\"name\":\"default\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn project_current_returns_the_active_project() {
        let (_dir, server) = test_server().await;
        let result = server.project_current().await.expect("project_current");
        assert!(
            text_of(&result).contains("\"name\":\"default\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn status_list_defaults_to_the_active_project() {
        let (_dir, server) = test_server().await;
        let result = server
            .status_list(Parameters(StatusListParams { project: None }))
            .await
            .expect("status_list");
        let text = text_of(&result);
        assert!(text.contains("\"name\":\"todo\""), "{text}");
        assert!(text.contains("\"name\":\"done\""), "{text}");
    }

    #[tokio::test]
    async fn status_list_reports_unknown_projects_as_tool_errors() {
        let (_dir, server) = test_server().await;
        let result = server
            .status_list(Parameters(StatusListParams {
                project: Some("nope".to_string()),
            }))
            .await
            .expect("status_list");
        assert_eq!(result.is_error, Some(true));
        let text = text_of(&result);
        assert!(text.contains("\"kind\":\"not_found\""), "{text}");
        assert!(text.contains("project 'nope' not found"), "{text}");
    }
}
