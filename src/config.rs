use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Config file name, used both in the current directory and under the XDG
/// config directory.
const CONFIG_FILE: &str = "scry.toml";

/// Legacy config file name, still read under the XDG config directory for
/// backwards compatibility.
const LEGACY_CONFIG_FILE: &str = "config.toml";

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
    /// Load configuration. The first file that exists wins:
    /// `./scry.toml` in the current working directory, then
    /// `$XDG_CONFIG_HOME/scry/scry.toml`, then the legacy
    /// `$XDG_CONFIG_HOME/scry/config.toml` (or their `~/.config/scry/`
    /// equivalents). With none present, the defaults apply. No file is ever
    /// created.
    ///
    /// Returns the default merged with the file's set properties.
    pub fn load() -> Result<Self, AppError> {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::load_from(&dir, &Self::xdg_dir())
    }

    /// Load from the first config file that exists under `dir` or `xdg_dir`.
    fn load_from(dir: &Path, xdg_dir: &Path) -> Result<Self, AppError> {
        let candidates = [
            dir.join(CONFIG_FILE),
            xdg_dir.join(CONFIG_FILE),
            xdg_dir.join(LEGACY_CONFIG_FILE),
        ];

        let Some(path) = candidates.into_iter().find(|path| path.is_file()) else {
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

    fn xdg_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir).join("scry")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config").join("scry")
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
            &dir.path().join(CONFIG_FILE),
            "database_url = \"sqlite://local.db\"\n",
        );
        let xdg = dir.path().join("xdg");
        write_config(
            &xdg.join(CONFIG_FILE),
            "database_url = \"sqlite://xdg.db\"\n",
        );

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://local.db");
    }

    #[test]
    fn xdg_scry_toml_is_used_without_a_local_file() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("xdg");
        write_config(
            &xdg.join(CONFIG_FILE),
            "database_url = \"sqlite://xdg.db\"\n",
        );

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://xdg.db");
    }

    #[test]
    fn preferred_name_wins_over_the_legacy_name() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("xdg");
        write_config(
            &xdg.join(CONFIG_FILE),
            "database_url = \"sqlite://new.db\"\n",
        );
        write_config(
            &xdg.join(LEGACY_CONFIG_FILE),
            "database_url = \"sqlite://legacy.db\"\n",
        );

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://new.db");
    }

    #[test]
    fn legacy_name_is_read_when_the_preferred_name_is_absent() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("xdg");
        write_config(
            &xdg.join(LEGACY_CONFIG_FILE),
            "database_url = \"sqlite://legacy.db\"\n",
        );

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.database_url, "sqlite://legacy.db");
    }

    #[test]
    fn defaults_apply_when_no_config_file_exists() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("xdg");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert!(
            config.database_url.contains("scry.db"),
            "{}",
            config.database_url
        );
    }
}
