use std::fmt;

use scry_core::error::StorageError;
use scry_core::models::TaskId;

#[derive(Debug, Clone)]
pub enum AppError {
    Usage(String),
    Config(String),
    Service(ServiceError),
    Storage(StorageError),

    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(msg) => write!(f, "usage error: {msg}"),
            AppError::Config(msg) => write!(f, "config error: {msg}"),
            AppError::Service(e) => write!(f, "{e}"),
            AppError::Storage(e) => write!(f, "storage error: {e}"),
            AppError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl AppError {
    /// Stable snake_case discriminator used for machine-readable error output.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Usage(_) => "invalid",
            AppError::Config(_) => "config",
            AppError::Internal(_) => "internal",
            AppError::Storage(error) => error.kind(),
            AppError::Service(error) => error.kind(),
        }
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        AppError::Storage(e)
    }
}

impl From<ServiceError> for AppError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::Storage(e) => AppError::Storage(e),
            _ => AppError::Service(e),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServiceError {
    StatusNotFound { status: String, project: String },
    TaskNotFound { task_id: TaskId, project: String },
    StatusNotEmpty { status: String, count: usize },
    StatusNameTaken { status: String },
    ProjectHasNoStatuses,
    TitleRequired,

    Storage(StorageError),
}

impl From<StorageError> for ServiceError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatusNotFound { status, project } => {
                write!(f, "Status \"{status}\" not found in \"{project}\"")
            }
            Self::TaskNotFound { task_id, project } => {
                write!(f, "Task {task_id} not found in \"{project}\"")
            }
            Self::StatusNotEmpty { status, count } => write!(
                f,
                "Cannot delete status with active tasks. Status \"{status}\" contains {count} tasks"
            ),
            Self::StatusNameTaken { status } => {
                write!(f, "Status \"{status}\" already exists in project")
            }
            Self::ProjectHasNoStatuses => write!(f, "Project has no statuses"),
            Self::TitleRequired => write!(f, "A task title is required"),
            Self::Storage(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl ServiceError {
    fn kind(&self) -> &'static str {
        match self {
            Self::StatusNotFound { .. } | Self::TaskNotFound { .. } => "not_found",
            Self::StatusNotEmpty { .. } | Self::StatusNameTaken { .. } => "conflict",
            Self::ProjectHasNoStatuses | Self::TitleRequired => "invalid",
            Self::Storage(error) => error.kind(),
        }
    }
}
