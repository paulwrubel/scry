//! The active project as client-local state, mirroring a kubectl context.
//!
//! It lives in `state.toml` under the XDG state directory rather than in the
//! backend, so each machine that talks to a shared server keeps its own active
//! project.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::ScryConfig;
use crate::error::AppError;

/// TOML file holding the client-local active project.
const STATE_FILE: &str = "state.toml";

/// Client-local state: which project is active on this machine.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClientState {
    pub active_project: Option<String>,
}

impl ClientState {
    /// Load the client state from its default location. A missing file is not an
    /// error; it yields the default state.
    pub fn load(config: &ScryConfig) -> Result<Self, AppError> {
        Self::load_from(&resolve_state_path(config.state_file.as_deref()))
    }

    /// Load the client state from `path`. A missing file yields the default state.
    fn load_from(path: &Path) -> Result<Self, AppError> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(AppError::Config(format!(
                    "failed to read {:?}: {}",
                    path, error
                )));
            }
        };

        toml::from_str(&content)
            .map_err(|error| AppError::Config(format!("failed to parse {:?}: {}", path, error)))
    }

    /// Write the client state to its default location, creating the parent
    /// directory first.
    pub fn save(&self, config: &ScryConfig) -> Result<(), AppError> {
        self.save_to(&resolve_state_path(config.state_file.as_deref()))
    }

    /// Make `name` the active project and persist it.
    pub fn set_active(config: &ScryConfig, name: &str) -> Result<(), AppError> {
        ClientState {
            active_project: Some(name.to_string()),
        }
        .save(config)
    }

    /// Write the client state to `path`, creating the parent directory first.
    fn save_to(&self, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AppError::Config(format!("failed to create {:?}: {}", parent, error))
            })?;
        }

        let content = toml::to_string(self).map_err(|error| {
            AppError::Config(format!("failed to serialize client state: {}", error))
        })?;

        std::fs::write(path, content)
            .map_err(|error| AppError::Config(format!("failed to write {:?}: {}", path, error)))
    }
}

/// Resolve the state file path, honoring a configured override when present.
fn resolve_state_path(configured: Option<&str>) -> PathBuf {
    match configured {
        Some(path) => expand_home(path),
        None => default_state_path(),
    }
}

/// Expand a leading `~` in `path` against the `HOME` environment variable.
fn expand_home(path: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    expand_home_with(path, Path::new(&home))
}

/// Expand a leading `~` in `path` against `home`.
fn expand_home_with(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// The default state file under `$XDG_STATE_HOME/scry/`, falling back to
/// `~/.local/state/scry/`.
fn default_state_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(dir).join("scry").join(STATE_FILE)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("scry")
            .join(STATE_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    #[test]
    fn missing_file_yields_the_default_state() {
        let dir = TempDir::new().expect("temp dir");

        let state = ClientState::load_from(&dir.path().join(STATE_FILE)).expect("load");

        assert!(state.active_project.is_none());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("nested").join(STATE_FILE);

        ClientState {
            active_project: Some("alpha".to_string()),
        }
        .save_to(&path)
        .expect("save");

        let loaded = ClientState::load_from(&path).expect("load");
        assert_eq!(loaded.active_project.as_deref(), Some("alpha"));
    }

    #[test]
    fn configured_state_file_is_used() {
        assert_eq!(
            resolve_state_path(Some("/tmp/x/state.toml")),
            PathBuf::from("/tmp/x/state.toml")
        );
    }

    #[test]
    fn configured_relative_state_file_is_kept_relative() {
        assert_eq!(
            resolve_state_path(Some("rel/state.toml")),
            PathBuf::from("rel/state.toml")
        );
    }

    #[test]
    fn leading_tilde_expands_to_home() {
        assert_eq!(
            expand_home_with("~/x/y.toml", Path::new("/home/u")),
            Path::new("/home/u/x/y.toml")
        );
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        assert_eq!(
            expand_home_with("~", Path::new("/home/u")),
            Path::new("/home/u")
        );
    }

    #[test]
    fn non_tilde_path_is_unchanged() {
        assert_eq!(
            expand_home_with("/a/b.toml", Path::new("/home/u")),
            Path::new("/a/b.toml")
        );
    }
}
