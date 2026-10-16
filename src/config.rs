use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Config file name, used both in the current directory and under the XDG
/// config directory.
const CONFIG_FILE: &str = "scry.toml";
/// Legacy config file name, still read under the XDG config directory for
/// backwards compatibility.
const LEGACY_CONFIG_FILE: &str = "config.toml";

/// Secret file name, discovered like `CONFIG_FILE` but holding credentials that
/// must not be committed.
const SECRET_FILE: &str = "scry.secret.toml";

/// Environment variable holding the database URL.
const DATABASE_URL_ENV: &str = "DATABASE_URL";
/// Environment variable holding the REST API bearer token.
const API_TOKEN_ENV: &str = "SCRY_API_TOKEN";
/// Environment variable holding the MCP bearer token.
const MCP_TOKEN_ENV: &str = "SCRY_MCP_TOKEN";

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ScryConfig {
    pub database_url: String,
    pub state_file: Option<String>,
}

impl Default for ScryConfig {
    fn default() -> Self {
        ScryConfig {
            database_url: Self::resolve_database_url(),
            state_file: None,
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
        if let Ok(url) = std::env::var(DATABASE_URL_ENV) {
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

/// Secrets loaded from `scry.secret.toml`, a file kept out of version control.
/// Its fields take precedence over `scry.toml` and the environment variables.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ScrySecrets {
    pub database_url: Option<String>,
    pub api_token: Option<String>,
    pub mcp_token: Option<String>,
}

impl ScrySecrets {
    /// Load secrets from the first `scry.secret.toml` that exists, using the
    /// same discovery as [`ScryConfig::load`]. With none present every field is
    /// unset. No file is ever created.
    pub fn load() -> Result<Self, AppError> {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::load_from(&dir, &ScryConfig::xdg_dir())
    }

    /// Load from the first secret file that exists under `dir` or `xdg_dir`.
    fn load_from(dir: &Path, xdg_dir: &Path) -> Result<Self, AppError> {
        let candidates = [dir.join(SECRET_FILE), xdg_dir.join(SECRET_FILE)];

        let Some(path) = candidates.into_iter().find(|path| path.is_file()) else {
            return Ok(Self::default());
        };

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Config(format!("failed to read {:?}: {}", path, e)))?;

        toml::from_str(&content)
            .map_err(|e| AppError::Config(format!("failed to parse {:?}: {}", path, e)))
    }

    /// Resolve the database URL: this file's value, else `config`'s.
    pub fn resolved_database_url(&self, config: &ScryConfig) -> String {
        self.database_url
            .clone()
            .unwrap_or_else(|| config.database_url.clone())
    }

    /// The REST token: this file's value, else `SCRY_API_TOKEN`.
    pub fn api_token(&self) -> Option<String> {
        self.api_token
            .clone()
            .or_else(|| non_empty_env(API_TOKEN_ENV))
    }

    /// The MCP token: this file's value, else `SCRY_MCP_TOKEN`.
    pub fn mcp_token(&self) -> Option<String> {
        self.mcp_token
            .clone()
            .or_else(|| non_empty_env(MCP_TOKEN_ENV))
    }
}

/// Read an environment variable, treating an empty value as unset.
fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
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

    #[test]
    fn local_secret_file_is_discovered() {
        let dir = TempDir::new().expect("temp dir");
        write_config(
            &dir.path().join(SECRET_FILE),
            "database_url = \"sqlite://secret.db\"\napi_token = \"rest-token\"\nmcp_token = \"mcp-token\"\n",
        );
        let xdg = dir.path().join("xdg");

        let secrets = ScrySecrets::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(secrets.database_url.as_deref(), Some("sqlite://secret.db"));
        assert_eq!(secrets.api_token.as_deref(), Some("rest-token"));
        assert_eq!(secrets.mcp_token.as_deref(), Some("mcp-token"));
    }

    #[test]
    fn secret_database_url_wins_over_scry_toml() {
        let dir = TempDir::new().expect("temp dir");
        write_config(
            &dir.path().join(CONFIG_FILE),
            "database_url = \"sqlite://config.db\"\n",
        );
        write_config(
            &dir.path().join(SECRET_FILE),
            "database_url = \"sqlite://secret.db\"\n",
        );
        let xdg = dir.path().join("xdg");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load config");
        let secrets = ScrySecrets::load_from(dir.path(), &xdg).expect("load secrets");
        assert_eq!(secrets.resolved_database_url(&config), "sqlite://secret.db");
    }

    #[test]
    fn missing_secret_file_yields_defaults_and_falls_back_to_config() {
        let dir = TempDir::new().expect("temp dir");
        let xdg = dir.path().join("xdg");
        write_config(
            &dir.path().join(CONFIG_FILE),
            "database_url = \"sqlite://config.db\"\n",
        );

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load config");
        let secrets = ScrySecrets::load_from(dir.path(), &xdg).expect("load secrets");
        assert!(secrets.database_url.is_none());
        assert!(secrets.api_token.is_none());
        assert!(secrets.mcp_token.is_none());
        assert_eq!(secrets.resolved_database_url(&config), "sqlite://config.db");
    }

    #[test]
    fn state_file_is_loaded_from_config() {
        let dir = TempDir::new().expect("temp dir");
        write_config(
            &dir.path().join(CONFIG_FILE),
            "state_file = \"/tmp/x/state.toml\"\n",
        );
        let xdg = dir.path().join("xdg");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert_eq!(config.state_file.as_deref(), Some("/tmp/x/state.toml"));
    }

    #[test]
    fn state_file_is_none_when_absent() {
        let dir = TempDir::new().expect("temp dir");
        write_config(
            &dir.path().join(CONFIG_FILE),
            "database_url = \"sqlite://local.db\"\n",
        );
        let xdg = dir.path().join("xdg");

        let config = ScryConfig::load_from(dir.path(), &xdg).expect("load");
        assert!(config.state_file.is_none());
    }
}
