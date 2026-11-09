//! HTTP REST API server.

mod auth;
mod handlers;
mod router;
mod state;

pub use router::serve_http;
pub use state::ScryState;
