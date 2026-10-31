use scry_core::error::StorageError;
use scry_core::ids::TaskId;
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
    SectionNotFound {
        section: String,
        project: String,
        suggestion: Option<String>,
    },
    TaskNotFound {
        task_id: TaskId,
        project: String,
    },
    SectionNameTaken {
        section: String,
    },
    SectionNameRequired,
    TitleRequired,
    TaskHandleNotFound {
        short_id: String,
    },
    NoteHandleNotFound {
        short_id: String,
    },
    AmbiguousTaskHandle {
        short_id: String,
        candidates: Vec<String>,
    },
    AmbiguousNoteHandle {
        short_id: String,
        candidates: Vec<String>,
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
            Self::SectionNotFound {
                section,
                project,
                suggestion,
            } => {
                write!(f, "Section \"{section}\" not found in \"{project}\".")?;
                if let Some(suggestion) = suggestion {
                    write!(f, " Did you mean \"{suggestion}\"?")?;
                }
                Ok(())
            }
            Self::TaskNotFound { task_id, project } => {
                write!(f, "Task {task_id} not found in \"{project}\"")
            }
            Self::SectionNameTaken { section } => {
                write!(f, "Section \"{section}\" already exists in this project.")
            }
            Self::SectionNameRequired => write!(f, "A section name is required"),
            Self::TitleRequired => write!(f, "A task title is required"),
            Self::TaskHandleNotFound { short_id } => {
                write!(f, "No task matches short id \"{short_id}\"")
            }
            Self::NoteHandleNotFound { short_id } => {
                write!(f, "No note matches short id \"{short_id}\"")
            }
            Self::AmbiguousTaskHandle {
                short_id,
                candidates,
            } => write!(
                f,
                "Short id \"{short_id}\" matches multiple tasks: {}. Use a longer prefix.",
                candidates.join(", ")
            ),
            Self::AmbiguousNoteHandle {
                short_id,
                candidates,
            } => write!(
                f,
                "Short id \"{short_id}\" matches multiple notes: {}. Use a longer prefix.",
                candidates.join(", ")
            ),
            Self::Storage(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl ServiceError {
    fn kind(&self) -> &'static str {
        match self {
            Self::SectionNotFound { .. } | Self::TaskNotFound { .. } => "not_found",
            Self::TaskHandleNotFound { .. } | Self::NoteHandleNotFound { .. } => "not_found",
            Self::SectionNameTaken { .. } => "conflict",
            Self::SectionNameRequired | Self::TitleRequired => "invalid",
            Self::AmbiguousTaskHandle { .. } | Self::AmbiguousNoteHandle { .. } => "invalid",
            Self::Storage(error) => error.kind(),
        }
    }
}
