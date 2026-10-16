//! The active project as client-local state, mirroring a kubectl context.
//!
//! It lives in `state.toml` under the XDG state directory rather than in the
//! backend, so each machine that talks to a shared server keeps its own active
//! project.

use scry_core::error::StorageError;
use scry_core::models::Project;
use scry_core::store::Store;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    pub fn load() -> Result<Self, AppError> {
        Self::load_from(&state_path())
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
    pub fn save(&self) -> Result<(), AppError> {
        self.save_to(&state_path())
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

/// The state file under `$XDG_STATE_HOME/scry/`, falling back to
/// `~/.local/state/scry/`.
#[cfg(not(test))]
fn state_path() -> PathBuf {
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

/// In tests, redirect to a per-process scratch file so the suite never reads or
/// writes the real user state.
#[cfg(test)]
fn state_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!("scry-test-state-{}", std::process::id()))
        .join(STATE_FILE)
}

/// Resolve the active project: the state file's project when it still exists,
/// otherwise the project named `default`. Errors when neither is present.
pub async fn active_project(store: &dyn Store) -> Result<Project, AppError> {
    let state = ClientState::load()?;

    if let Some(name) = &state.active_project
        && let Some(project) = store.get_project_by_name(name).await?
    {
        return Ok(project);
    }

    store
        .get_project_by_name("default")
        .await?
        .ok_or_else(|| AppError::Internal("project 'default' not found".to_string()))
}

/// Make `name` the active project, verifying it exists first.
pub async fn set_active_project(store: &dyn Store, name: &str) -> Result<Project, AppError> {
    let project = store.get_project_by_name(name).await?.ok_or_else(|| {
        AppError::Storage(StorageError::NotFound(format!(
            "project '{}' not found",
            name
        )))
    })?;

    ClientState {
        active_project: Some(name.to_string()),
    }
    .save()?;

    Ok(project)
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
}
