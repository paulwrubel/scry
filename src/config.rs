use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ScryConfig {
    pub database_url: String,
}

impl Default for ScryConfig {
    fn default() -> Self {
        ScryConfig {
            database_url: Self::resolve_database_url(),
        }
    }
}

impl ScryConfig {
    /// Load config from `$XDG_CONFIG_HOME/scry/config.toml` (or `~/.config/scry/config.toml`).
    /// Returns the default merged with the file's set properties, if any.
    pub fn load() -> Result<Self, AppError> {
        let path = Self::path();

        if !path.exists() {
            return Ok(ScryConfig::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Config(format!("failed to read {:?}: {}", path, e)))?;

        toml::from_str(&content)
            .map_err(|e| AppError::Config(format!("failed to parse {:?}: {}", path, e)))
    }

    fn resolve_database_url() -> String {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            return url;
        }
        let dir = if let Ok(d) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(d).join("scry")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("scry")
        };
        format!("sqlite://{}", dir.join("scry.db").display())
    }

    fn path() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir).join("scry").join("config.toml")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".config")
                .join("scry")
                .join("config.toml")
        }
    }
}
