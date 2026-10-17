//! In-crate tests for the REST API, driving the assembled router directly.

use assert_fs::TempDir;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::serve;
use scry_core::models::{Priority, StatusStyle, Tags, TaskSortingMode};
use scry_core::store::{ArcStore, Store, TaskToCreate};
use scry_remote::RemoteStore;
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
    (
        status,
        String::from_utf8(bytes.to_vec()).expect("utf-8 body"),
    )
}

#[tokio::test]
async fn index_reports_the_server_name_and_version() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(&app, request("GET", "/api/v1", None, None)).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"scry\""), "{body}");
    assert!(body.contains(env!("CARGO_PKG_VERSION")), "{body}");
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
        request(
            "POST",
            "/api/v1/projects",
            None,
            Some(r#"{"name":"alpha"}"#),
        ),
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

#[tokio::test]
async fn remote_store_round_trips_against_the_server() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener address");
    tokio::spawn(async move {
        let _ = serve(listener, app).await;
    });

    let store = RemoteStore::new(&format!("http://{addr}"), None)
        .await
        .expect("remote store");

    let project = store
        .create_project(
            "remote-demo".to_string(),
            None,
            TaskSortingMode::Manual,
            false,
        )
        .await
        .expect("create project");

    let todo = store
        .create_status(
            project.id,
            "todo".to_string(),
            0,
            None,
            StatusStyle::Unchecked,
        )
        .await
        .expect("create todo status");
    let done = store
        .create_status(
            project.id,
            "done".to_string(),
            1,
            None,
            StatusStyle::Checked,
        )
        .await
        .expect("create done status");

    let task = store
        .create_task(TaskToCreate {
            project_id: project.id,
            title: "ship it".to_string(),
            description: None,
            priority: Priority::Medium,
            status_id: todo.id,
            position: 0,
            tags: Tags::default(),
        })
        .await
        .expect("create task");
    assert_eq!(task.status_id, todo.id);

    let listed = store
        .get_all_tasks_by_project_id(project.id)
        .await
        .expect("list tasks");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, task.id);

    let task_id = task.id;
    let mut moved = task;
    moved.status_id = done.id;
    let moved = store
        .update_and_autoposition_task(moved)
        .await
        .expect("move task");
    assert_eq!(moved.status_id, done.id);

    let fetched = store
        .get_task_by_id(task_id)
        .await
        .expect("get task")
        .expect("task is present");
    assert_eq!(fetched.status_id, done.id);

    store
        .delete_project("remote-demo".to_string())
        .await
        .expect("delete project");

    assert!(
        store
            .get_project_by_name("remote-demo")
            .await
            .expect("look up deleted project")
            .is_none()
    );
}

#[tokio::test]
async fn project_snapshot_returns_the_whole_project() {
    let (_dir, state) = test_state().await;
    let store = state.store.clone();
    let app = router::build_router(state, None);

    let project = store
        .create_project("state-demo".to_string(), None, TaskSortingMode::Manual, false)
        .await
        .expect("create project");
    let todo = store
        .create_status(
            project.id,
            "todo".to_string(),
            0,
            None,
            StatusStyle::Unchecked,
        )
        .await
        .expect("create todo status");
    let task = store
        .create_task(TaskToCreate {
            project_id: project.id,
            title: "ship it".to_string(),
            description: None,
            priority: Priority::Medium,
            status_id: todo.id,
            position: 0,
            tags: Tags::default(),
        })
        .await
        .expect("create task");
    store
        .create_note(task.id, "remember this".to_string())
        .await
        .expect("create note");

    let (status, body) = send(
        &app,
        request(
            "GET",
            &format!("/api/v1/projects/{}/snapshot", project.id),
            None,
            None,
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let value = serde_json::from_str::<serde_json::Value>(&body).expect("json body");
    assert_eq!(value["project"]["name"], "state-demo");
    assert_eq!(value["statuses"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["tasks"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["notes"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["notes"][0]["contents"], "remember this");
}

#[tokio::test]
async fn project_snapshot_for_missing_project_is_not_found() {
    let (_dir, state) = test_state().await;
    let app = router::build_router(state, None);

    let (status, body) = send(
        &app,
        request("GET", "/api/v1/projects/999/snapshot", None, None),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body.contains("\"kind\":\"not_found\""), "{body}");
}
