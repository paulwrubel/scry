//! Browser session routes.

use axum::Router;
use axum::extract::Extension;
use axum::routing::get;

use crate::http_auth::AuthState;

use super::handlers::{create_session, delete_session, session_status};

/// Routes under `/api/v1/auth`.
pub(super) fn build_auth_router<S>(auth: AuthState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/session",
            get(session_status)
                .post(create_session)
                .delete(delete_session),
        )
        .layer(Extension(auth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::response::Response;
    use axum_extra::extract::cookie::Cookie;
    use tower::ServiceExt;

    fn session_app(auth: AuthState) -> Router {
        build_auth_router::<()>(auth)
    }

    fn session_request(token: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(format!(r#"{{"token":"{token}"}}"#)))
            .unwrap()
    }

    fn cookie_from(response: &Response) -> String {
        let raw = response.headers()[header::SET_COOKIE].to_str().unwrap();
        let cookie = Cookie::parse(raw.to_string()).unwrap();
        format!("{}={}", cookie.name(), cookie.value())
    }

    fn delete_request() -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri("/session")
            .body(Body::empty())
            .unwrap()
    }

    fn status_request() -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri("/session")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn session_endpoint_sets_the_cookie() {
        let app = session_app(AuthState::new(Some("secret".into())));
        let response = app.oneshot(session_request("secret")).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.contains("scry_session="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Strict"));
        assert!(set_cookie.contains("Path=/"));
        assert!(set_cookie.contains("Max-Age=2592000"));
        assert!(!set_cookie.contains("Secure"));
    }

    #[tokio::test]
    async fn session_endpoint_sets_secure_cookie_behind_tls() {
        let app = session_app(AuthState::new(Some("secret".into())));
        let request = Request::builder()
            .method("POST")
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-proto", "https")
            .body(Body::from(r#"{"token":"secret"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.contains("Secure"));
    }

    #[tokio::test]
    async fn wrong_token_is_rejected() {
        let app = session_app(AuthState::new(Some("secret".into())));
        let response = app.oneshot(session_request("wrong")).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
    }

    #[tokio::test]
    async fn delete_clears_the_cookie() {
        let auth = AuthState::new(Some("secret".into()));
        let created = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();
        let cookie = cookie_from(&created);

        let request = Request::builder()
            .method("DELETE")
            .uri("/session")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap();

        let response = session_app(auth).oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.contains("scry_session="));
        assert!(set_cookie.contains("Max-Age=0"));
    }

    #[tokio::test]
    async fn disabled_auth_rejects_session_creation() {
        let app = session_app(AuthState::new(None));
        let response = app.oneshot(session_request("secret")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn disabled_auth_still_clears() {
        let app = session_app(AuthState::new(None));
        let response = app.oneshot(delete_request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn session_status_rejects_without_a_cookie() {
        let app = session_app(AuthState::new(Some("secret".into())));
        let response = app.oneshot(status_request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn session_status_accepts_a_created_session() {
        let auth = AuthState::new(Some("secret".into()));
        let created = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();
        let cookie = cookie_from(&created);

        let request = Request::builder()
            .method("GET")
            .uri("/session")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap();

        let response = session_app(auth).oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn session_status_accepts_when_auth_is_disabled() {
        let app = session_app(AuthState::new(None));
        let response = app.oneshot(status_request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn cookie_authenticates_api_requests() {
        let auth = AuthState::new(Some("secret".into()));
        let response = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();
        let cookie = cookie_from(&response);

        let app = crate::http_auth::require_auth(
            Router::new().route("/", axum::routing::get(|| async { "ok" })),
            auth.clone(),
        );
        let request = Request::builder()
            .uri("/")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap();

        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rotating_the_token_rejects_the_old_cookie() {
        let old_auth = AuthState::new(Some("old".into()));
        let response = session_app(old_auth)
            .oneshot(session_request("old"))
            .await
            .unwrap();
        let cookie = cookie_from(&response);

        let app = crate::http_auth::require_auth(
            Router::new().route("/", axum::routing::get(|| async { "ok" })),
            AuthState::new(Some("new".into())),
        );
        let request = Request::builder()
            .uri("/")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
