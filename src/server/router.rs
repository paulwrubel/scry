//! Router assembly and HTTP startup for the REST API.

use crate::error::AppError;
use crate::http_auth::require_token;
use axum::Router;
use axum::http::{Method, header};
use axum::routing::{get, post};
use axum::serve;
use scry_core::store::ArcStore;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use super::{ScryState, handlers};

/// Environment variable holding an optional bearer token API clients must present.
const TOKEN_ENV: &str = "SCRY_API_TOKEN";

/// CORS layer permitting browsers to call the API from any origin.
fn get_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(Any)
}

/// Routes nested under the versioned `/api/v1` prefix.
fn build_api_router() -> Router<ScryState> {
    Router::new()
        .route("/", get(handlers::index))
        .route(
            "/active-project",
            get(handlers::get_active_project).put(handlers::set_active_project),
        )
        .route("/import", post(handlers::import))
        .nest("/projects", build_projects_router())
        .nest("/statuses", build_statuses_router())
        .nest("/tasks", build_tasks_router())
        .nest("/notes", build_notes_router())
}

/// Routes nested under `/projects`.
fn build_projects_router() -> Router<ScryState> {
    Router::new()
        .route(
            "/",
            get(handlers::get_projects).post(handlers::create_project),
        )
        .route(
            "/by-name/{name}",
            get(handlers::get_project_by_name).delete(handlers::delete_project),
        )
        .route(
            "/{id}",
            get(handlers::get_project)
                .put(handlers::update_project)
                .delete(handlers::delete_project_by_id),
        )
        .route(
            "/{project_id}/statuses",
            get(handlers::get_statuses).post(handlers::create_status),
        )
        .route(
            "/{project_id}/tasks",
            get(handlers::get_tasks_by_project).post(handlers::create_task),
        )
        .route("/{project_id}/notes", get(handlers::get_notes_by_project))
        .route(
            "/{project_id}/statuses/by-name/{name}",
            get(handlers::get_status_by_name),
        )
}

/// Routes nested under `/statuses`.
fn build_statuses_router() -> Router<ScryState> {
    Router::new()
        .route(
            "/{id}",
            get(handlers::get_status)
                .put(handlers::update_status)
                .delete(handlers::delete_status),
        )
        .route("/{id}/move", post(handlers::move_status))
        .route("/{id}/tasks", get(handlers::get_tasks_by_status))
}

/// Routes nested under `/tasks`.
fn build_tasks_router() -> Router<ScryState> {
    Router::new()
        .route(
            "/{id}",
            get(handlers::get_task)
                .put(handlers::update_task)
                .delete(handlers::delete_task),
        )
        .route("/{id}/move", post(handlers::move_task))
        .route(
            "/{id}/notes",
            get(handlers::get_notes_by_task).post(handlers::create_note),
        )
}

/// Routes nested under `/notes`.
fn build_notes_router() -> Router<ScryState> {
    Router::new().route(
        "/{id}",
        get(handlers::get_note)
            .put(handlers::update_note)
            .delete(handlers::delete_note),
    )
}

/// Assemble the full router: the versioned API, CORS, shared state, and the
/// optional bearer-token gate.
pub fn build_router(state: ScryState, token: Option<String>) -> Router {
    let router = Router::new()
        .nest("/api/v1", build_api_router())
        .layer(get_cors_layer())
        .with_state(state);
    require_token(router, token)
}

/// Serve the REST API over HTTP on `addr`.
pub async fn serve_http(store: ArcStore, addr: &str) -> Result<(), AppError> {
    let token = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|token| !token.is_empty());

    let addr: SocketAddr = addr
        .parse()
        .map_err(|error| AppError::Usage(format!("invalid address '{addr}': {error}")))?;

    let state = ScryState { store };
    let router = build_router(state, token);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| AppError::Internal(format!("failed to bind {addr}: {error}")))?;

    serve(listener, router)
        .await
        .map_err(|error| AppError::Internal(format!("REST server error: {error}")))?;

    Ok(())
}
