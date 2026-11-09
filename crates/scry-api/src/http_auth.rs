//! Shared bearer-token authentication for scry's HTTP surfaces.
//!
//! The static `admin_password` remains the login and CLI credential. Browser
//! clients exchange it for a short-lived access JWT plus a database-backed
//! refresh token; only `Authorization: Bearer` may authenticate ordinary API
//! requests.

use crate::auth_store::{AuthStore, NewRefreshToken, RefreshRotation};
use axum::Router;
use axum::extract::FromRequestParts;
use axum::http::{HeaderMap, StatusCode, request::Parts};
use axum::middleware::from_extractor_with_state;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::extract::cookie::{Cookie, Key, SameSite};
use axum_extra::headers::{Authorization, authorization::Bearer};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use time::Duration;
use uuid::Uuid;

/// Name of the signed browser refresh-token cookie.
pub(crate) const REFRESH_COOKIE: &str = "refresh_token";
const REFRESH_MAX_AGE_DAYS: i64 = 30;
const ACCESS_TOKEN_SECONDS: i64 = 60 * 60;
const SUBJECT: &str = "local";

/// Credentials shared by the API gate and the auth endpoints.
#[derive(Clone)]
pub(crate) struct AuthState {
    admin_password: Option<String>,
    key: Key,
    access_secret: Option<Vec<u8>>,
    refresh_tokens: Arc<dyn AuthStore>,
}

impl AuthState {
    pub(crate) fn new(admin_password: Option<String>, refresh_tokens: Arc<dyn AuthStore>) -> Self {
        let (key, access_secret) = match admin_password.as_deref() {
            // Deriving both from the password means rotating it invalidates
            // every cookie and access token at once.
            Some(admin_password) => {
                let digest = Sha256::digest(admin_password.as_bytes());
                (Key::derive_from(digest.as_slice()), Some(digest.to_vec()))
            }
            None => (Key::generate(), None),
        };

        Self {
            admin_password,
            key,
            access_secret,
            refresh_tokens,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.admin_password.is_some()
    }

    pub(crate) fn key(&self) -> Key {
        self.key.clone()
    }

    pub(crate) fn check_admin_password(&self, presented: &str) -> bool {
        let Some(admin_password) = self.admin_password.as_deref() else {
            return false;
        };
        presented.as_bytes().ct_eq(admin_password.as_bytes()).into()
    }

    /// Issue a one-hour HS256 access token for the local subject.
    pub(crate) fn issue_access_token(&self) -> Result<String, AuthError> {
        self.issue_access_token_for(SUBJECT)
    }

    fn issue_access_token_for(&self, subject: &str) -> Result<String, AuthError> {
        let secret = self.access_secret.as_deref().ok_or(AuthError::Disabled)?;
        let now = Utc::now().timestamp();
        let claims = AccessClaims {
            sub: subject.to_string(),
            iat: now,
            exp: now + ACCESS_TOKEN_SECONDS,
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .map_err(|error| AuthError::Internal(format!("failed to issue access token: {error}")))
    }

    /// Validate an access JWT and return the subject it names.
    pub(crate) fn validate_access_token(&self, token: &str) -> Option<Principal> {
        let secret = self.access_secret.as_deref()?;
        let validation = Validation::new(Algorithm::HS256);
        let data =
            decode::<AccessClaims>(token, &DecodingKey::from_secret(secret), &validation).ok()?;

        Some(Principal {
            subject: data.claims.sub,
        })
    }

    /// Create a refresh token and return its raw value for the cookie.
    pub(crate) async fn create_refresh_token(&self) -> Result<String, AuthError> {
        if !self.enabled() {
            return Err(AuthError::Disabled);
        }

        let raw = generate_refresh_token();
        let now = Utc::now();

        self.refresh_tokens
            .create_refresh_token(NewRefreshToken {
                token_hash: hash_refresh_token(&raw),
                subject: SUBJECT.to_string(),
                origin_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now + chrono::TimeDelta::days(REFRESH_MAX_AGE_DAYS),
            })
            .await
            .map_err(internal)?;

        Ok(raw)
    }

    /// Rotate `raw` and return a new access JWT plus refresh token, or `None`
    /// when the refresh token is missing, expired, revoked, or reused.
    pub(crate) async fn rotate_refresh_token(
        &self,
        raw: &str,
    ) -> Result<Option<(String, String)>, AuthError> {
        if !self.enabled() {
            return Err(AuthError::Disabled);
        }

        let new_raw = generate_refresh_token();
        let now = Utc::now();

        let rotation = self
            .refresh_tokens
            .rotate_refresh_token(
                &hash_refresh_token(raw),
                &hash_refresh_token(&new_raw),
                now,
                now + chrono::TimeDelta::days(REFRESH_MAX_AGE_DAYS),
            )
            .await
            .map_err(internal)?;

        match rotation {
            RefreshRotation::Rotated { subject } => {
                let access_token = self.issue_access_token_for(&subject)?;
                Ok(Some((access_token, new_raw)))
            }
            RefreshRotation::Missing
            | RefreshRotation::Expired
            | RefreshRotation::Revoked
            | RefreshRotation::Reused => Ok(None),
        }
    }

    /// Revoke `raw` if it exists. Revoking a missing or already-revoked token is
    /// not an error so logout is idempotent.
    pub(crate) async fn revoke_refresh_token(&self, raw: &str) -> Result<(), AuthError> {
        self.refresh_tokens
            .revoke_refresh_token(&hash_refresh_token(raw))
            .await
            .map_err(internal)
    }
}

/// Claims carried by an access JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccessClaims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

/// The authenticated caller attached to request extensions by `RequireAuth`.
#[derive(Debug, Clone)]
pub(crate) struct Principal {
    #[allow(dead_code)]
    pub subject: String,
}

/// Why an auth endpoint or extraction failed.
#[derive(Debug)]
pub(crate) enum AuthError {
    Disabled,
    Unauthorized,
    Internal(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::Disabled => (
                StatusCode::BAD_REQUEST,
                "authentication is disabled; no admin password is configured",
            )
                .into_response(),
            AuthError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "missing or invalid credentials").into_response()
            }
            AuthError::Internal(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
        }
    }
}

fn internal(error: impl std::fmt::Display) -> AuthError {
    AuthError::Internal(format!("refresh token store error: {error}"))
}

fn generate_refresh_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_refresh_token(raw: &str) -> Vec<u8> {
    Sha256::digest(raw.as_bytes()).to_vec()
}

/// Extractor that admits a request carrying the static admin password or a
/// valid access JWT in `Authorization: Bearer`. Cookies are deliberately ignored
/// here.
struct RequireAuth;

impl FromRequestParts<AuthState> for RequireAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AuthState,
    ) -> Result<Self, Self::Rejection> {
        let bearer =
            match TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await {
                Ok(TypedHeader(Authorization(bearer))) => bearer,
                Err(_) => return Err(unauthorized()),
            };

        if state.check_admin_password(bearer.token()) {
            parts.extensions.insert(Principal {
                subject: SUBJECT.to_string(),
            });
            return Ok(RequireAuth);
        }

        if let Some(principal) = state.validate_access_token(bearer.token()) {
            parts.extensions.insert(principal);
            return Ok(RequireAuth);
        }

        Err(unauthorized())
    }
}

/// The rejection returned for missing or invalid credentials.
fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "missing or invalid credentials").into_response()
}

/// Gate `router` behind bearer credentials.
///
/// Returns the router unchanged when no admin password is configured. Otherwise
/// every API request must present the bearer token or a valid access JWT.
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

/// The signed refresh cookie set by `POST /api/v1/auth/session` and refreshed
/// by `POST /api/v1/auth/refresh`.
pub(crate) fn refresh_cookie(raw_token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, raw_token))
        .path("/api/v1/auth")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::days(REFRESH_MAX_AGE_DAYS))
        .build()
}

/// The expired cookie used to clear a refresh token.
pub(crate) fn cleared_refresh_cookie() -> Cookie<'static> {
    Cookie::build(REFRESH_COOKIE).path("/api/v1/auth").build()
}

/// Whether the request reached the server over TLS, as reported by the proxy.
pub(crate) fn request_is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_store::test_support::MemoryAuthStore;
    use axum::body::Body;
    use axum::extract::Extension;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::get;
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

    async fn echo_subject(Extension(principal): Extension<Principal>) -> String {
        principal.subject
    }

    fn app(auth: AuthState) -> Router {
        Router::new()
            .route("/", get(|| async { "ok" }))
            .route("/whoami", get(echo_subject))
            .layer(from_extractor_with_state::<RequireAuth, _>(auth))
    }

    async fn status(app: Router, request: Request<Body>) -> StatusCode {
        app.oneshot(request).await.unwrap().status()
    }

    fn request_with(name: header::HeaderName, value: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header(name, value)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn static_bearer_authenticates() {
        let (auth, _) = auth_state(Some("secret"));
        let request = request_with(header::AUTHORIZATION, "Bearer secret");

        assert_eq!(status(app(auth), request).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn access_token_authenticates() {
        let (auth, _) = auth_state(Some("secret"));
        let token = auth.issue_access_token().unwrap();
        let request = request_with(header::AUTHORIZATION, &format!("Bearer {token}"));

        assert_eq!(status(app(auth), request).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_bearer_is_rejected() {
        let (auth, _) = auth_state(Some("secret"));
        let request = request_with(header::AUTHORIZATION, "Bearer wrong");

        assert_eq!(status(app(auth), request).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refresh_cookie_alone_does_not_authenticate() {
        let (auth, _) = auth_state(Some("secret"));
        let request = request_with(header::COOKIE, "refresh_token=raw-token");

        assert_eq!(status(app(auth), request).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn principal_subject_is_inserted_into_extensions() {
        let (auth, _) = auth_state(Some("secret"));
        let request = Request::builder()
            .uri("/whoami")
            .header(header::AUTHORIZATION, "Bearer secret")
            .body(Body::empty())
            .unwrap();

        let response = app(auth).oneshot(request).await.unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(&body[..], b"local");
    }

    #[tokio::test]
    async fn rotating_admin_password_rejects_the_old_access_token() {
        let (old, _) = auth_state(Some("old"));
        let token = old.issue_access_token().unwrap();

        let (new, _) = auth_state(Some("new"));
        let request = request_with(header::AUTHORIZATION, &format!("Bearer {token}"));

        assert_eq!(status(app(new), request).await, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn refresh_cookie_has_the_agreed_attributes() {
        let cookie = refresh_cookie("raw-token".to_string(), false);

        assert_eq!(cookie.name(), REFRESH_COOKIE);
        assert_eq!(cookie.value(), "raw-token");
        assert_eq!(cookie.path(), Some("/api/v1/auth"));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        assert_eq!(cookie.max_age(), Some(Duration::days(30)));
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(refresh_cookie("raw".to_string(), true).secure(), Some(true));
    }

    #[test]
    fn cleared_refresh_cookie_targets_the_refresh_path() {
        let cookie = cleared_refresh_cookie();

        assert_eq!(cookie.name(), REFRESH_COOKIE);
        assert_eq!(cookie.path(), Some("/api/v1/auth"));
    }

    #[test]
    fn refresh_tokens_are_64_hex_characters() {
        let raw = generate_refresh_token();

        assert_eq!(raw.len(), 64);
        assert!(raw.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn no_token_disables_auth() {
        let (auth, _) = auth_state(None);

        assert!(!auth.enabled());
    }

    #[test]
    fn secure_detection_uses_forwarded_proto() {
        assert!(!request_is_secure(&HeaderMap::new()));

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        assert!(request_is_secure(&headers));

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "HTTPS, http".parse().unwrap());
        assert!(request_is_secure(&headers));

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "http".parse().unwrap());
        assert!(!request_is_secure(&headers));
    }
}
