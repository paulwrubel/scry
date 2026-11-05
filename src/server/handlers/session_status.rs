//! Handler for reporting browser session status.

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::http_auth::AuthState;

/// Report whether the browser already has a valid session cookie.
///
/// The endpoint is ungated so the SPA can decide between the app and login.
/// With authentication disabled the API is usable, so it reports success.
pub async fn session_status(Extension(auth): Extension<AuthState>, headers: HeaderMap) -> Response {
    if !auth.enabled() || auth.has_valid_session(&headers) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
