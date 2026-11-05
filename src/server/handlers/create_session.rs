//! Handler for creating a browser session.

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;
use serde::Deserialize;

use crate::http_auth::{AuthState, request_is_secure, session_cookie};

#[derive(Deserialize)]
pub struct SessionRequest {
    token: String,
}

/// Exchange the configured API token for a signed session cookie.
pub async fn create_session(
    Extension(auth): Extension<AuthState>,
    headers: HeaderMap,
    Json(body): Json<SessionRequest>,
) -> Response {
    if !auth.enabled() {
        return (
            StatusCode::BAD_REQUEST,
            "authentication is disabled; no api_token is configured",
        )
            .into_response();
    }

    if !auth.check_token(&body.token) {
        return (StatusCode::UNAUTHORIZED, "missing or invalid token").into_response();
    }

    let jar = SignedCookieJar::from_headers(&headers, auth.key());
    (
        StatusCode::NO_CONTENT,
        jar.add(session_cookie(request_is_secure(&headers))),
    )
        .into_response()
}
