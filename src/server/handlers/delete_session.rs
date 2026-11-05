//! Handler for clearing a browser session.

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;

use crate::http_auth::{AuthState, cleared_session_cookie};

/// Clear the signed session cookie.
pub async fn delete_session(Extension(auth): Extension<AuthState>, headers: HeaderMap) -> Response {
    let jar = SignedCookieJar::from_headers(&headers, auth.key());
    (StatusCode::NO_CONTENT, jar.remove(cleared_session_cookie())).into_response()
}
