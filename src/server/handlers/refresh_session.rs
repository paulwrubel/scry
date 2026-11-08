//! Handler for rotating a browser refresh token.

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;

use crate::http_auth::{
    AuthState, REFRESH_COOKIE, cleared_refresh_cookie, refresh_cookie, request_is_secure,
};

use super::create_session::AccessTokenResponse;

/// Rotate the signed refresh cookie, returning a new access JWT and cookie.
pub async fn refresh_session(
    Extension(auth): Extension<AuthState>,
    headers: HeaderMap,
) -> Response {
    let jar = SignedCookieJar::from_headers(&headers, auth.key());

    let raw = jar
        .get(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string());

    let Some(raw) = raw else {
        return (
            StatusCode::UNAUTHORIZED,
            jar.remove(cleared_refresh_cookie()),
        )
            .into_response();
    };

    match auth.rotate_refresh_token(&raw).await {
        Ok(Some((access_token, refresh_token))) => (
            StatusCode::OK,
            jar.add(refresh_cookie(refresh_token, request_is_secure(&headers))),
            Json(AccessTokenResponse { access_token }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            jar.remove(cleared_refresh_cookie()),
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}
