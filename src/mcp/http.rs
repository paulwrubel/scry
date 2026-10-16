//! Streamable HTTP transport for the MCP server.

use crate::error::AppError;
use crate::http_auth::require_token;
use axum::Router;
use axum::serve;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use scry_core::store::ArcStore;
use std::net::SocketAddr;

use super::ScryServer;

/// Environment variable holding an optional bearer token HTTP clients must present.
const TOKEN_ENV: &str = "SCRY_MCP_TOKEN";

/// Browser origins permitted to reach the endpoint (DNS-rebinding protection).
const LOOPBACK_ORIGINS: &[&str] = &["http://localhost:*", "http://127.0.0.1:*", "http://[::1]:*"];

/// Build the Axum router serving the MCP endpoint at `/mcp`.
///
/// Sessions are disabled and simple request-response calls are answered with
/// JSON. Browser origins are restricted to loopback, and an optional bearer
/// token gates every request.
pub(crate) fn router(store: &ArcStore, token: Option<String>) -> Router {
    let store = store.clone();

    // A non-loopback bind is legitimate (see `serve_http`), so the loopback-only
    // Host allow-list is disabled; the bearer token and the Origin allow-list
    // remain the gate.
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_allowed_origins(LOOPBACK_ORIGINS.iter().copied())
        .disable_allowed_hosts();

    let service = StreamableHttpService::new(
        move || Ok(ScryServer::new(store.clone())),
        LocalSessionManager::default().into(),
        config,
    );

    let router = Router::new().nest_service("/mcp", service);

    require_token(router, token)
}

/// Serve the MCP protocol over streamable HTTP on `addr`.
pub async fn serve_http(store: ArcStore, addr: &str) -> Result<(), AppError> {
    let token = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|token| !token.is_empty());

    let addr: SocketAddr = addr
        .parse()
        .map_err(|error| AppError::Usage(format!("invalid address '{addr}': {error}")))?;

    let router = router(&store, token);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| AppError::Internal(format!("failed to bind {addr}: {error}")))?;

    serve(listener, router)
        .await
        .map_err(|error| AppError::Internal(format!("HTTP server error: {error}")))?;

    Ok(())
}
