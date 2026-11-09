//! Shared response shapes for batch task operations.

use scry_core::models::Task;
use serde::Serialize;

use crate::error::AppError;

/// One outcome in a batch task response.
#[derive(Serialize)]
pub struct BatchTaskResult {
    pub handle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BatchTaskError>,
}

impl BatchTaskResult {
    /// A handle that resolved and whose operation succeeded.
    pub fn success(handle: String, task: Task) -> Self {
        Self {
            handle,
            task: Some(task),
            error: None,
        }
    }

    /// A handle that failed to resolve or whose operation failed.
    pub fn failure(handle: String, error: AppError) -> Self {
        Self {
            handle,
            task: None,
            error: Some(BatchTaskError {
                kind: error.kind().to_string(),
                message: error.to_string(),
                details: error.details(),
            }),
        }
    }
}

/// The machine-readable error for one failed batch item.
#[derive(Serialize)]
pub struct BatchTaskError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
