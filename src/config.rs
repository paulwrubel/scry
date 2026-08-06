use serde::Deserialize;
use std::path::PathBuf;

use crate::error::AppError;

const DEFAULT_CONFIG_TEMPLATE: &str = r##"# scry configuration
# See DESIGN.md for full documentation.

# Override the database URL (default: SQLite in XDG_DATA_HOME)
# database_url = "postgres://user:pass@localhost/scry"
"##;

#[derive(Debug, Deserialize)]
pub struct ScryConfig {
    /// Override for the DATABASE_URL environment variable.
    /// If set, this takes precedence.
    #[serde(default)]
    pub database_url: Option<String>,
}

impl ScryConfig {
    /// Load config from `$XDG_CONFIG_HOME/scry/config.toml` (or `~/.config/scry/config.toml`).
    /// Returns a default config with all fields set to `None` if the file does not exist.
    pub fn load() -> Result<Self, AppError> {
        let path = config_path();

        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Config(format!("failed to create config directory: {}", e))
                })?;
            }
            let template = DEFAULT_CONFIG_TEMPLATE;
            std::fs::write(&path, template)
                .map_err(|e| AppError::Config(format!("failed to write default config: {}", e)))?;
            return Ok(ScryConfig { database_url: None });
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Config(format!("failed to read {:?}: {}", path, e)))?;

        toml::from_str(&content)
            .map_err(|e| AppError::Config(format!("failed to parse {:?}: {}", path, e)))
    }

    /// Resolve the effective database URL: config file override > DATABASE_URL env var > default SQLite path.
    pub fn resolve_database_url(&self) -> String {
        if let Some(ref url) = self.database_url {
            return url.clone();
        }
        if let Ok(url) = std::env::var("DATABASE_URL") {
            return url;
        }
        // default SQLite path
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
}

fn config_path() -> PathBuf {
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
