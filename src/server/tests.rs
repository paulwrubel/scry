//! In-crate tests for the REST API, driving the assembled router directly.

use assert_fs::TempDir;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use scry_core::store::ArcStore;
use scry_sqlite::SqliteStore;
use tower::ServiceExt;

use super::{ScryState, router};
use crate::error::AppError;

async fn test_state() -> (TempDir, ScryState) {
    let dir = TempDir::new().expect("temp dir");
    let url = format!("sqlite://{}", dir.path().join("scry.db").display());
    let store: ArcStore = std::sync::Arc::new(SqliteStore::new(&url).await.expect("store"));
    (dir, ScryState { store })
}

/// Build a request, attaching a bearer token and/or a JSON body when given.
fn request(method: &str, uri: &str, token: Option<&str>, body: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    match body {
        Some(json) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .expect("build request"),
        None => builder.body(Body::empty()).expect("build request"),
    }
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.expect("send request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    (status, String::from_utf8(bytes.to_vec()).expect("utf-8 body"))
}

#[tokio::test]
async fn index_reports_the_active_project() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(&app, request("GET", "/api/v1", None, None)).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"scry\""), "{body}");
    assert!(body.contains("\"default\""), "{body}");
}

#[tokio::test]
async fn unmatched_route_returns_404() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, _) = send(&app, request("GET", "/api/v1/nope", None, None)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn no_token_configured_allows_requests() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(&app, request("GET", "/api/v1", None, None)).await;

    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn token_is_required_when_configured() {
    let (_dir, state) = test_state().await;
    let token = "0123456789abcdef";
    let app = router::build_router(state, Some(token.to_string()));

    let (status, _) = send(&app, request("GET", "/api/v1", None, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = send(&app, request("GET", "/api/v1", Some(token), None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _) = send(&app, request("GET", "/api/v1", Some("wrong-token"), None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_and_list_project_round_trips() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(
        &app,
        request("POST", "/api/v1/projects", None, Some(r#"{"name":"alpha"}"#)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"alpha\""), "{body}");

    let (status, body) = send(&app, request("GET", "/api/v1/projects", None, None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"alpha\""), "{body}");
}

#[tokio::test]
async fn get_missing_project_by_name_is_null() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(
        &app,
        request("GET", "/api/v1/projects/by-name/ghost", None, None),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, "null");
}

#[test]
fn app_error_maps_to_http_status() {
    let status = Response::from(AppError::Usage("x".to_string()))
        .into_response()
        .status();
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let status = Response::from(AppError::Config("x".to_string()))
        .into_response()
        .status();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

    let status = Response::from(AppError::Internal("x".to_string()))
        .into_response()
        .status();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}
