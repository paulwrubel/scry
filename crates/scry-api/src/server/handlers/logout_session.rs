//! Handler for logging out a browser session.

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;

use crate::http_auth::{AuthState, REFRESH_COOKIE, cleared_refresh_cookie};

/// Revoke the refresh token if present and clear the cookie. Logging out
/// without a cookie is a no-op so the endpoint is idempotent.
pub async fn logout_session(Extension(auth): Extension<AuthState>, headers: HeaderMap) -> Response {
    let jar = SignedCookieJar::from_headers(&headers, auth.key());

    let raw = jar
        .get(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string());

    let revoke_error = match raw {
        Some(raw) => auth.revoke_refresh_token(&raw).await.err(),
        None => None,
    };

    if let Some(error) = revoke_error {
        return error.into_response();
    }

    (StatusCode::NO_CONTENT, jar.remove(cleared_refresh_cookie())).into_response()
}
