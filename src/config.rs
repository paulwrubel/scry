use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Config file read from the current working directory, if present.
const LOCAL_CONFIG_FILE: &str = "scry.toml";

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
    /// Load configuration.
    ///
    /// `./scry.toml` in the current working directory takes precedence; when it
    /// is absent, `$XDG_CONFIG_HOME/scry/config.toml`` is used. With neither present, the defaults
    /// apply.
    ///
    /// Returns the default merged with the file's set properties.
    pub fn load() -> Result<Self, AppError> {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::load_from(&dir, &Self::xdg_path())
    }

    /// Load from the first config file that exists under `dir` or at `xdg_path`.
    fn load_from(dir: &Path, xdg_path: &Path) -> Result<Self, AppError> {
        let local = dir.join(LOCAL_CONFIG_FILE);
        let path = if local.is_file() {
            local
        } else if xdg_path.is_file() {
            xdg_path.to_path_buf()
        } else {
            return Ok(ScryConfig::default());
        };

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

    fn xdg_path() -> PathBuf {
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    fn write_config(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, contents).expect("write config");
    }

    #[test]
    fn local_scry_toml_takes_precedence_over_xdg() {
        let dir = TempDir::new().expect("temp dir");
        write_config(
            &dir.path().join(LOCAL_CONFIG_FILE),
            "database_url = \"sqlite://local.db\"\n",
        );
        let xdg = dir.path().join("scry").join("config.toml");
        write_config(&xdg, "database_url = \"sqlite://xdg.db\"\n");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://local.db");
    }

    #[test]
    fn xdg_file_is_used_without_a_local_scry_toml() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("config.toml");
        write_config(&xdg, "database_url = \"sqlite://xdg.db\"\n");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://xdg.db");
    }

    #[test]
    fn defaults_apply_when_neither_file_exists() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("missing.toml");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert!(
            config.database_url.contains("scry.db"),
            "{}",
            config.database_url
        );
    }
}
