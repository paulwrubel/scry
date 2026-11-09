//! Browser session routes.

use axum::Router;
use axum::extract::Extension;
use axum::routing::post;

use crate::http_auth::AuthState;

use super::handlers::{create_session, logout_session, refresh_session};

/// Routes under `/api/v1/auth`.
pub(super) fn build_auth_router<S>(auth: AuthState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/session", post(create_session))
        .route("/refresh", post(refresh_session))
        .route("/logout", post(logout_session))
        .layer(Extension(auth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_store::AuthStore;
    use crate::auth_store::test_support::MemoryAuthStore;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::response::Response;
    use axum_extra::extract::cookie::Cookie;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn auth_state(admin_password: Option<&str>) -> (AuthState, Arc<MemoryAuthStore>) {
        let store = Arc::new(MemoryAuthStore::new());
        let refresh_tokens: Arc<dyn AuthStore> = store.clone();
        (
            AuthState::new(admin_password.map(str::to_string), refresh_tokens),
            store,
        )
    }

    fn session_app(auth: AuthState) -> Router {
        build_auth_router::<()>(auth)
    }

    fn session_request(password: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(format!(r#"{{"password":"{password}"}}"#)))
            .unwrap()
    }

    fn refresh_request(cookie: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().method("POST").uri("/refresh");
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        builder.body(Body::empty()).unwrap()
    }

    fn logout_request(cookie: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/logout")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap()
    }

    fn cookie_from(response: &Response) -> String {
        let raw = response.headers()[header::SET_COOKIE].to_str().unwrap();
        let cookie = Cookie::parse(raw.to_string()).unwrap();
        format!("{}={}", cookie.name(), cookie.value())
    }

    fn set_cookie(response: &Response) -> String {
        response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_string()
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn login_returns_an_access_token_and_sets_the_cookie() {
        let (auth, _) = auth_state(Some("secret"));
        let response = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let cookie = set_cookie(&response);
        assert!(cookie.contains("refresh_token="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/api/v1/auth"));
        assert!(cookie.contains("Max-Age=2592000"));
        assert!(!cookie.contains("Secure"));

        let body = json_body(response).await;
        let access_token = body["access_token"].as_str().unwrap();
        assert!(auth.validate_access_token(access_token).is_some());
    }

    #[tokio::test]
    async fn login_sets_a_secure_cookie_behind_tls() {
        let (auth, _) = auth_state(Some("secret"));
        let request = Request::builder()
            .method("POST")
            .uri("/session")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-proto", "https")
            .body(Body::from(r#"{"password":"secret"}"#))
            .unwrap();

        let response = session_app(auth).oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(set_cookie(&response).contains("Secure"));
    }

    #[tokio::test]
    async fn wrong_password_is_rejected() {
        let (auth, _) = auth_state(Some("secret"));
        let response = session_app(auth)
            .oneshot(session_request("wrong"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
    }

    #[tokio::test]
    async fn disabled_auth_rejects_session_creation() {
        let (auth, _) = auth_state(None);
        let response = session_app(auth)
            .oneshot(session_request("secret"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn refresh_rotates_the_cookie_and_old_cookie_reuse_fails() {
        let (auth, _) = auth_state(Some("secret"));
        let created = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();
        let old_cookie = cookie_from(&created);

        let refreshed = session_app(auth.clone())
            .oneshot(refresh_request(Some(old_cookie.as_str())))
            .await
            .unwrap();
        assert_eq!(refreshed.status(), StatusCode::OK);
        let new_cookie = cookie_from(&refreshed);
        assert_ne!(old_cookie, new_cookie);

        let body = json_body(refreshed).await;
        assert!(
            auth.validate_access_token(body["access_token"].as_str().unwrap())
                .is_some()
        );

        let reused = session_app(auth)
            .oneshot(refresh_request(Some(old_cookie.as_str())))
            .await
            .unwrap();
        assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);
        assert!(set_cookie(&reused).contains("Max-Age=0"));
    }

    #[tokio::test]
    async fn logout_revokes_the_token_and_clears_the_cookie() {
        let (auth, _) = auth_state(Some("secret"));
        let created = session_app(auth.clone())
            .oneshot(session_request("secret"))
            .await
            .unwrap();
        let cookie = cookie_from(&created);

        let response = session_app(auth.clone())
            .oneshot(logout_request(&cookie))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let cleared = set_cookie(&response);
        assert!(cleared.contains("refresh_token="));
        assert!(cleared.contains("Max-Age=0"));

        let refreshed = session_app(auth)
            .oneshot(refresh_request(Some(cookie.as_str())))
            .await
            .unwrap();
        assert_eq!(refreshed.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_refresh_cookie_is_rejected() {
        let (auth, _) = auth_state(Some("secret"));
        let response = session_app(auth)
            .oneshot(refresh_request(None))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn disabled_auth_still_clears_on_logout() {
        let (auth, _) = auth_state(None);
        let request = Request::builder()
            .method("POST")
            .uri("/logout")
            .body(Body::empty())
            .unwrap();

        let response = session_app(auth).oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
