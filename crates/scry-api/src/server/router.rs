//! Router assembly and HTTP startup for the REST API.

use crate::auth_store::AuthStore;
use crate::error::AppError;
use crate::http_auth::{AuthState, require_auth};
use crate::service::ProjectService;
use axum::Router;
use axum::http::{HeaderName, Method, StatusCode, header};
use axum::routing::{get, post};
use axum::serve;
#[cfg(feature = "embed-ui")]
use include_dir::{Dir, include_dir};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use super::{ScryState, handlers};

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
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("time-zone"),
        ])
        .allow_origin(Any)
}

/// Routes nested under the versioned `/api/v1` prefix.
fn build_api_router() -> Router<ScryState> {
    Router::new()
        .route("/", get(handlers::index))
        .route("/export", get(handlers::export))
        .route("/snapshot", get(handlers::snapshot))
        .route("/import", post(handlers::import))
        .route("/max_seq", get(handlers::max_seq))
        .route("/sync", post(handlers::sync))
        .nest("/projects", build_projects_router())
        .nest("/sections", build_sections_router())
        .nest("/tasks", build_tasks_router())
        .nest("/notes", build_notes_router())
        .nest("/automations", build_automations_router())
        .fallback(not_found)
}

/// Answer unmatched API paths with a bare 404 instead of falling through to the
/// UI, which would otherwise serve the app shell for a missing endpoint.
async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
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
            get(handlers::get_project_by_name)
                .put(handlers::update_project_by_name)
                .delete(handlers::delete_project_by_name),
        )
        .route(
            "/{id}",
            get(handlers::get_project)
                .put(handlers::update_project)
                .delete(handlers::delete_project_by_id),
        )
        .route("/{id}/export", get(handlers::export_project))
        .route(
            "/{project_id}/sections",
            get(handlers::get_sections).post(handlers::create_section),
        )
        .route(
            "/{project_id}/sections/by-name/{name}",
            get(handlers::get_section_by_name),
        )
        .route(
            "/{project_id}/tasks",
            get(handlers::get_tasks_by_project).post(handlers::create_task),
        )
        .route("/{project_id}/notes", get(handlers::get_notes_by_project))
        .route(
            "/{project_id}/automations",
            get(handlers::get_automations).post(handlers::create_automation),
        )
        .route(
            "/{project_id}/snapshot",
            get(handlers::get_project_snapshot),
        )
}

/// Routes nested under `/sections`.
fn build_sections_router() -> Router<ScryState> {
    Router::new()
        .route(
            "/{id}",
            get(handlers::get_section)
                .put(handlers::update_section)
                .delete(handlers::delete_section),
        )
        .route("/{id}/merge", post(handlers::merge_section))
        .route("/{id}/move", post(handlers::move_section))
        .route("/{id}/tasks", get(handlers::get_tasks_by_section))
}

/// Routes nested under `/tasks`.
fn build_tasks_router() -> Router<ScryState> {
    Router::new()
        .route("/close", post(handlers::close_tasks))
        .route("/open", post(handlers::open_tasks))
        .route(
            "/{id}",
            get(handlers::get_task)
                .put(handlers::update_task)
                .delete(handlers::delete_task),
        )
        .route("/{id}/move", post(handlers::move_task))
        .route("/{id}/duplicate", post(handlers::duplicate_task))
        .route("/{id}/close", post(handlers::close_task))
        .route("/{id}/open", post(handlers::open_task))
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

/// Routes nested under `/automations`.
fn build_automations_router() -> Router<ScryState> {
    Router::new().route(
        "/{id}",
        get(handlers::get_automation)
            .put(handlers::update_automation)
            .delete(handlers::delete_automation),
    )
}

#[cfg(feature = "embed-ui")]
static UI_DIST_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../ui/dist");

#[cfg(feature = "embed-ui")]
fn get_content_type(path: &str) -> &'static str {
    match path.split('.').next_back().unwrap_or("") {
        "html" => "text/html",
        "js" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "text/plain",
    }
}

/// Hashed Vite assets never change; everything else must be revalidated so the
/// service worker and index can update.
#[cfg(feature = "embed-ui")]
fn cache_control(path: &str) -> &'static str {
    if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

#[cfg(feature = "embed-ui")]
fn build_ui_router() -> Router<ScryState> {
    Router::new().fallback(|req: axum::http::Request<axum::body::Body>| async move {
        let path = req.uri().path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };

        if let Some(file) = UI_DIST_DIR.get_file(path) {
            axum::response::Response::builder()
                .header("content-type", get_content_type(path))
                .header("cache-control", cache_control(path))
                .body(axum::body::Body::from(file.contents()))
                .unwrap()
        } else {
            // If file not found, return index.html for client-side routing
            if let Some(index_file) = UI_DIST_DIR.get_file("index.html") {
                axum::response::Response::builder()
                    .header("content-type", "text/html")
                    .header("cache-control", "no-cache")
                    .body(axum::body::Body::from(index_file.contents()))
                    .unwrap()
            } else {
                axum::response::Response::builder()
                    .status(404)
                    .body(axum::body::Body::from("Not found"))
                    .unwrap()
            }
        }
    })
}

/// Assemble the full router: the versioned API behind the optional credential
/// gate, the ungated auth endpoints, CORS, shared state, and the public UI.
pub fn build_router(
    state: ScryState,
    auth_store: Arc<dyn AuthStore>,
    admin_password: Option<String>,
) -> Router {
    let auth = AuthState::new(admin_password, auth_store);
    let resources = require_auth(build_api_router(), auth.clone());
    let api = Router::new()
        .merge(resources)
        .nest("/auth", super::auth::build_auth_router(auth));
    let router = Router::new().nest("/api/v1", api).layer(get_cors_layer());

    #[cfg(feature = "embed-ui")]
    let router = router.merge(build_ui_router());

    router.with_state(state)
}

/// Serve the REST API over HTTP on `addr`.
pub async fn serve_http(
    service: ProjectService,
    auth_store: Arc<dyn AuthStore>,
    addr: &str,
    admin_password: Option<String>,
) -> Result<(), AppError> {
    let addr: SocketAddr = addr
        .parse()
        .map_err(|error| AppError::Usage(format!("invalid address '{addr}': {error}")))?;

    let state = ScryState { service };
    let router = build_router(state, auth_store, admin_password);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| AppError::Internal(format!("failed to bind {addr}: {error}")))?;

    serve(listener, router)
        .await
        .map_err(|error| AppError::Internal(format!("REST server error: {error}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use tower::ServiceExt;

    #[tokio::test]
    async fn cors_does_not_allow_credentials() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(get_cors_layer());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/")
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .is_none()
        );
    }
}

#[cfg(all(test, feature = "embed-ui"))]
mod ui_asset_tests {
    use super::*;

    #[test]
    fn webmanifest_uses_the_manifest_mime_type() {
        assert_eq!(
            get_content_type("manifest.webmanifest"),
            "application/manifest+json"
        );
    }

    #[test]
    fn hashed_assets_are_cached_immutably() {
        assert_eq!(
            cache_control("assets/index-B954sXMp.js"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn shell_files_are_revalidated() {
        assert_eq!(cache_control("index.html"), "no-cache");
        assert_eq!(cache_control("sw.js"), "no-cache");
        assert_eq!(cache_control("manifest.webmanifest"), "no-cache");
        assert_eq!(cache_control("pwa-192x192.png"), "no-cache");
    }
}
