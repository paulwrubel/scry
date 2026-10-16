//! Shared bearer-token authentication for scry's HTTP surfaces.

use axum::Router;
use axum::extract::FromRequestParts;
use axum::http::{StatusCode, request::Parts};
use axum::middleware::from_extractor_with_state;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::{Authorization, authorization::Bearer};
use subtle::ConstantTimeEq;

/// The expected bearer token, carried as the gate's layer state.
#[derive(Clone)]
struct TokenState {
    token: String,
}

/// Extractor that admits a request only when it presents the configured bearer token.
///
/// Deployed as a router-wide layer through [`require_token`], mirroring luxide's
/// `Claims` extractor. The presented token is compared in constant time.
struct RequireToken;

impl FromRequestParts<TokenState> for RequireToken {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &TokenState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| unauthorized())?;

        let authorized = bearer
            .token()
            .as_bytes()
            .ct_eq(state.token.as_bytes())
            .into();

        if authorized {
            Ok(RequireToken)
        } else {
            Err(unauthorized())
        }
    }
}

/// The rejection returned for a missing or invalid token.
fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "missing or invalid bearer token").into_response()
}

/// Gate `router` behind the bearer token when one is configured.
///
/// Returns the router unchanged when no token is configured; otherwise every
/// request must present the token or be rejected.
pub(crate) fn require_token(router: Router, token: Option<String>) -> Router {
    match token {
        Some(token) => router.layer(from_extractor_with_state::<RequireToken, _>(TokenState {
            token,
        })),
        None => router,
    }
}
