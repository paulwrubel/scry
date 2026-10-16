//! Shared state threaded through the REST API handlers.

use scry_core::store::ArcStore;

/// Shared state every REST handler operates on: the database store.
#[derive(Clone)]
pub struct ScryState {
    pub store: ArcStore,
}
