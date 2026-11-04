//! Shared bearer-token and session-cookie authentication for scry's HTTP surfaces.

use axum::Router;
use axum::extract::FromRequestParts;
use axum::http::{StatusCode, request::Parts};
use axum::middleware::from_extractor_with_state;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Key;
use axum_extra::headers::{Authorization, authorization::Bearer};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Name of the interim browser session cookie.
pub(crate) const SESSION_COOKIE: &str = "scry_session";
const SESSION_VALUE: &str = "authenticated";

/// Credentials shared by the API gate and the session endpoints.
#[derive(Clone)]
pub(crate) struct AuthState {
    token: Option<String>,
    key: Key,
}

impl AuthState {
    pub(crate) fn new(token: Option<String>) -> Self {
        let key = match token.as_deref() {
            // Deriving from the token means rotating it invalidates every cookie.
            Some(token) => {
                let digest = Sha256::digest(token.as_bytes());
                Key::derive_from(digest.as_slice())
            }
            None => Key::generate(),
        };

        Self { token, key }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.token.is_some()
    }

    pub(crate) fn key(&self) -> Key {
        self.key.clone()
    }

    pub(crate) fn check_token(&self, presented: &str) -> bool {
        self.token
            .as_deref()
            .is_some_and(|token| presented.as_bytes().ct_eq(token.as_bytes()).into())
    }
}

/// Extractor that admits a request with the configured bearer token or a valid
/// signed session cookie.
struct RequireAuth;

impl FromRequestParts<AuthState> for RequireAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AuthState,
    ) -> Result<Self, Self::Rejection> {
        if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            return if state.check_token(bearer.token()) {
                Ok(RequireAuth)
            } else {
                Err(unauthorized())
            };
        }

        let jar = SignedCookieJar::from_headers(&parts.headers, state.key());
        match jar.get(SESSION_COOKIE) {
            Some(cookie) if cookie.value() == SESSION_VALUE => Ok(RequireAuth),
            _ => Err(unauthorized()),
        }
    }
}

/// The rejection returned for missing or invalid credentials.
fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "missing or invalid credentials").into_response()
}

/// Gate `router` behind the configured credentials.
///
/// Returns the router unchanged when no token is configured. Otherwise every
/// API request must present the bearer token or a valid session cookie.
pub(crate) fn require_auth<S>(router: Router<S>, auth: AuthState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if auth.enabled() {
        router.layer(from_extractor_with_state::<RequireAuth, _>(auth))
    } else {
        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::get;
    use axum_extra::extract::cookie::Cookie;
    use tower::ServiceExt;

    fn app(auth: AuthState) -> Router {
        Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(from_extractor_with_state::<RequireAuth, _>(auth))
    }

    async fn status(app: Router, request: Request<Body>) -> StatusCode {
        app.oneshot(request).await.unwrap().status()
    }

    fn cookie_header(auth: &AuthState) -> String {
        let cookie = Cookie::build((SESSION_COOKIE, SESSION_VALUE))
            .path("/")
            .build();
        let jar = SignedCookieJar::new(auth.key()).add(cookie);
        let response = (StatusCode::NO_CONTENT, jar).into_response();
        let raw = response.headers()[header::SET_COOKIE].to_str().unwrap();
        let cookie = Cookie::parse(raw.to_string()).unwrap();

        format!("{}={}", cookie.name(), cookie.value())
    }

    fn request_with(name: header::HeaderName, value: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header(name, value)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn bearer_token_authenticates() {
        let app = app(AuthState::new(Some("secret".into())));
        let request = request_with(header::AUTHORIZATION, "Bearer secret");

        assert_eq!(status(app, request).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn session_cookie_authenticates() {
        let auth = AuthState::new(Some("secret".into()));
        let request = request_with(header::COOKIE, &cookie_header(&auth));
        let app = app(auth);

        assert_eq!(status(app, request).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_credentials_are_rejected() {
        let app = app(AuthState::new(Some("secret".into())));
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        assert_eq!(status(app, request).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_bearer_is_rejected() {
        let app = app(AuthState::new(Some("secret".into())));
        let request = request_with(header::AUTHORIZATION, "Bearer wrong");

        assert_eq!(status(app, request).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rotated_token_rejects_the_old_cookie() {
        let old_cookie = cookie_header(&AuthState::new(Some("old".into())));
        let app = app(AuthState::new(Some("new".into())));
        let request = request_with(header::COOKIE, &old_cookie);

        assert_eq!(status(app, request).await, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn no_token_disables_auth() {
        assert!(!AuthState::new(None).enabled());
    }
}
