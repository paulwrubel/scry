//! Streamable HTTP transport for the MCP server.

use crate::error::AppError;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use scry_core::store::ArcStore;
use subtle::ConstantTimeEq;

use super::ScryServer;

/// Environment variable holding an optional bearer token HTTP clients must present.
const TOKEN_ENV: &str = "SCRY_MCP_TOKEN";

/// Browser origins permitted to reach the endpoint (DNS-rebinding protection).
const LOOPBACK_ORIGINS: &[&str] = &["http://localhost:*", "http://127.0.0.1:*", "http://[::1]:*"];

/// Build the Axum router serving the MCP endpoint at `/mcp`.
///
/// Sessions are disabled and simple request-response calls are answered with
/// JSON. Browser origins are restricted to loopback, an optional bearer token
/// gates every request, and `allow_any_host` relaxes Host validation for
/// non-loopback bindings.
pub(crate) fn router(
    store: &ArcStore,
    token: Option<String>,
    allow_any_host: bool,
) -> axum::Router {
    let store = store.clone();

    let mut config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_allowed_origins(LOOPBACK_ORIGINS.iter().copied());
    if allow_any_host {
        config = config.disable_allowed_hosts();
    }

    let service = StreamableHttpService::new(
        move || Ok(ScryServer::new(store.clone())),
        LocalSessionManager::default().into(),
        config,
    );

    let router = axum::Router::new().nest_service("/mcp", service);

    match token {
        Some(token) => router.layer(axum::middleware::from_fn_with_state(
            TokenState { token },
            require_bearer,
        )),
        None => router,
    }
}

#[derive(Clone)]
struct TokenState {
    token: String,
}

/// Reject requests that do not present the configured bearer token.
async fn require_bearer(State(state): State<TokenState>, request: Request, next: Next) -> Response {
    let authorized = bearer_token(request.headers())
        .is_some_and(|presented| presented.as_bytes().ct_eq(state.token.as_bytes()).into());

    if authorized {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, "missing or invalid bearer token").into_response()
    }
}

/// Extract the token from an `Authorization: Bearer <token>` header.
///
/// The scheme is matched case-insensitively, per RFC 6750.
fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("bearer")
        .then_some(token.trim_start())
}

/// Serve the MCP protocol over streamable HTTP on `addr`.
///
/// The address decides exposure: a non-loopback bind is reachable from other
/// hosts, so it emits a warning and disables the loopback Host allow-list.
pub async fn serve_http(store: ArcStore, addr: &str) -> Result<(), AppError> {
    let token = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|token| !token.is_empty());

    let loopback = host_is_loopback(addr);
    if !loopback {
        eprintln!(
            "warning: {addr} is reachable from other hosts; {}",
            match token {
                Some(_) => "requests must present the SCRY_MCP_TOKEN bearer token",
                None => "set SCRY_MCP_TOKEN to require a bearer token",
            }
        );
    }

    let router = router(&store, token, !loopback);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| AppError::Internal(format!("failed to bind {addr}: {error}")))?;

    axum::serve(listener, router)
        .await
        .map_err(|error| AppError::Internal(format!("HTTP server error: {error}")))?;

    Ok(())
}

/// Whether the host portion of `addr` is loopback or `localhost`.
pub(crate) fn host_is_loopback(addr: &str) -> bool {
    use std::net::IpAddr;

    let host = addr
        .rsplit_once(':')
        .map(|(host, _port)| host)
        .unwrap_or(addr)
        .trim_start_matches('[')
        .trim_end_matches(']');

    match host.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => host.eq_ignore_ascii_case("localhost"),
    }
}
