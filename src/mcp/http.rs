//! Streamable HTTP transport for the MCP server.

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::error::AppError;
use crate::store::sqlite::SqliteStore;

use super::ScryServer;

/// Build the Axum router serving the MCP endpoint at `/mcp`.
///
/// Sessions are disabled and simple request-response calls are answered with
/// JSON, which keeps a local single-user server stateless and easy to drive.
pub(crate) fn router(store: &SqliteStore) -> axum::Router {
    let store = store.clone();
    let service = StreamableHttpService::new(
        move || Ok(ScryServer::new(store.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true),
    );

    axum::Router::new().nest_service("/mcp", service)
}

/// Serve the MCP protocol over streamable HTTP on `addr`.
pub async fn serve_http(store: SqliteStore, addr: &str) -> Result<(), AppError> {
    let router = router(&store);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| AppError::Internal(format!("failed to bind {addr}: {error}")))?;

    axum::serve(listener, router)
        .await
        .map_err(|error| AppError::Internal(format!("HTTP server error: {error}")))?;

    Ok(())
}
