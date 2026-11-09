//! Handler for creating a browser session.

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;
use serde::{Deserialize, Serialize};

use crate::http_auth::{AuthError, AuthState, refresh_cookie, request_is_secure};

#[derive(Deserialize)]
pub struct SessionRequest {
    password: String,
}

/// The short-lived access token returned by login and refresh.
#[derive(Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
}

/// Exchange the configured admin password for an access JWT and a refresh cookie.
pub async fn create_session(
    Extension(auth): Extension<AuthState>,
    headers: HeaderMap,
    Json(body): Json<SessionRequest>,
) -> Response {
    if !auth.enabled() {
        return AuthError::Disabled.into_response();
    }

    if !auth.check_admin_password(&body.password) {
        return AuthError::Unauthorized.into_response();
    }

    let access_token = match auth.issue_access_token() {
        Ok(token) => token,
        Err(error) => return error.into_response(),
    };
    let refresh_token = match auth.create_refresh_token().await {
        Ok(token) => token,
        Err(error) => return error.into_response(),
    };

    let jar = SignedCookieJar::from_headers(&headers, auth.key());
    (
        StatusCode::OK,
        jar.add(refresh_cookie(refresh_token, request_is_secure(&headers))),
        Json(AccessTokenResponse { access_token }),
    )
        .into_response()
}
