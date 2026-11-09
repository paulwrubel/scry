use scry_core::config::ConfigError;
use std::fmt;

/// Errors produced by the scry CLI.
#[derive(Debug, Clone)]
pub enum AppError {
    Usage(String),
    Config(String),
    Update(String),

    Remote {
        kind: &'static str,
        message: String,
        details: Option<serde_json::Value>,
    },

    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(msg) => write!(f, "usage error: {msg}"),
            AppError::Config(msg) => write!(f, "config error: {msg}"),
            AppError::Update(msg) => write!(f, "update error: {msg}"),
            AppError::Remote { message, .. } => write!(f, "remote error: {message}"),
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
            AppError::Update(_) => "update",
            AppError::Internal(_) => "internal",
            AppError::Remote { kind, .. } => kind,
        }
    }

    /// Structured details for errors that carry more than a message.
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            AppError::Remote { details, .. } => details.clone(),
            _ => None,
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        AppError::Config(error.to_string())
    }
}
