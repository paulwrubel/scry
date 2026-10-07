//! In-process Model Context Protocol (MCP) server exposing scry operations as tools.

use crate::error::AppError;
use crate::store::TaskStore;
use crate::store::sqlite::SqliteStore;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use serde::Serialize;

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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ScryServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "scry is a terminal task manager. Use these tools to manage tasks, projects, statuses, and notes.",
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
}
