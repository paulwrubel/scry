use std::fmt;

#[derive(Debug, Clone)]
pub enum StorageError {
    Database(String),
    NotFound(String),
    Conflict(String),
    Invalid(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Database(msg) => write!(f, "database error: {msg}"),
            StorageError::NotFound(msg) => write!(f, "not found: {msg}"),
            StorageError::Conflict(msg) => write!(f, "conflict: {msg}"),
            StorageError::Invalid(msg) => write!(f, "invalid operation: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl StorageError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "internal",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Invalid(_) => "invalid",
        }
    }
}
