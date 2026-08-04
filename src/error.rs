use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Store(StorageError),
    Config(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Store(e) => write!(f, "store error: {e}"),
            AppError::Config(msg) => write!(f, "config error: {msg}"),
            AppError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        AppError::Store(e)
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

#[derive(Debug)]
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
