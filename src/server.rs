//! HTTP REST API server.

mod handlers;
mod router;
mod state;

pub use router::serve_http;
pub use state::ScryState;

#[cfg(test)]
mod tests;
