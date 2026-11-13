use scry_core::config::ConfigError;
use scry_core::error::StorageError;
use scry_core::ids::{AutomationId, TaskId};
use std::fmt;

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

    /// Structured details for errors that carry more than a message.
    ///
    /// Kept separate from `Display` so machine-readable output can expose the
    /// candidate list without callers parsing prose.
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            AppError::Service(ServiceError::AmbiguousTaskHandle { candidates, .. })
            | AppError::Service(ServiceError::AmbiguousCommentHandle { candidates, .. }) => {
                Some(serde_json::json!({ "candidates": candidates }))
            }
            _ => None,
        }
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        AppError::Storage(e)
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        AppError::Config(error.to_string())
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
    SectionNotFound {
        section: String,
        project: String,
    },
    TaskNotFound {
        task_id: TaskId,
        project: String,
    },
    SectionNameTaken {
        section: String,
    },
    SectionMergeIntoItself {
        section: String,
    },
    SectionMoveAnchorItself {
        section: String,
    },
    TaskMoveAnchorItself {
        task: TaskId,
    },
    TaskAnchorNotInSection {
        task: TaskId,
        section: String,
    },
    SectionNameRequired,
    TaskHandleNotFound {
        handle: String,
    },
    CommentHandleNotFound {
        handle: String,
    },
    AmbiguousTaskHandle {
        handle: String,
        candidates: Vec<String>,
    },
    AmbiguousCommentHandle {
        handle: String,
        candidates: Vec<String>,
    },
    SectionInUseByAutomation {
        section: String,
    },
    AutomationNotFound {
        id: AutomationId,
    },
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
            Self::SectionNotFound { section, project } => {
                write!(f, "Section \"{section}\" not found in \"{project}\".")
            }
            Self::TaskNotFound { task_id, project } => {
                write!(f, "Task {task_id} not found in \"{project}\"")
            }
            Self::SectionNameTaken { section } => {
                write!(f, "Section \"{section}\" already exists in this project.")
            }
            Self::SectionMergeIntoItself { section } => {
                write!(f, "Section \"{section}\" cannot be merged into itself")
            }
            Self::SectionMoveAnchorItself { section } => {
                write!(
                    f,
                    "Section \"{section}\" cannot be moved relative to itself."
                )
            }
            Self::TaskMoveAnchorItself { task } => {
                write!(f, "Task {task} cannot be moved relative to itself")
            }
            Self::TaskAnchorNotInSection { task, section } => {
                write!(f, "Task {task} is not in section \"{section}\"")
            }
            Self::SectionNameRequired => write!(f, "A section name is required"),
            Self::TaskHandleNotFound { handle } => {
                write!(f, "No task matches handle \"{handle}\"")
            }
            Self::CommentHandleNotFound { handle } => {
                write!(f, "No comment matches handle \"{handle}\"")
            }
            Self::AmbiguousTaskHandle { handle, candidates } => write!(
                f,
                "Handle \"{handle}\" matches multiple tasks: {}. Use a longer prefix.",
                candidates.join(", ")
            ),
            Self::AmbiguousCommentHandle { handle, candidates } => write!(
                f,
                "Handle \"{handle}\" matches multiple comments: {}. Use a longer prefix.",
                candidates.join(", ")
            ),
            Self::SectionInUseByAutomation { section } => write!(
                f,
                "Section \"{section}\" is used by an automation. Remove or change that automation first."
            ),
            Self::AutomationNotFound { id } => write!(f, "Automation {id} not found."),
            Self::Storage(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl ServiceError {
    fn kind(&self) -> &'static str {
        match self {
            Self::SectionNotFound { .. } | Self::TaskNotFound { .. } => "not_found",
            Self::TaskHandleNotFound { .. } | Self::CommentHandleNotFound { .. } => "not_found",
            Self::AutomationNotFound { .. } => "not_found",
            Self::SectionNameTaken { .. }
            | Self::SectionMergeIntoItself { .. }
            | Self::SectionMoveAnchorItself { .. }
            | Self::TaskMoveAnchorItself { .. } => "conflict",
            Self::SectionInUseByAutomation { .. } => "conflict",
            Self::SectionNameRequired => "invalid",
            Self::TaskAnchorNotInSection { .. } => "invalid",
            Self::AmbiguousTaskHandle { .. } | Self::AmbiguousCommentHandle { .. } => "invalid",
            Self::Storage(error) => error.kind(),
        }
    }
}
