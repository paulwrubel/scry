//! Shared state threaded through the REST API handlers.

use crate::service::ProjectService;

/// Shared state every REST handler operates on: the project service.
#[derive(Clone)]
pub struct ScryState {
    pub service: ProjectService,
}
