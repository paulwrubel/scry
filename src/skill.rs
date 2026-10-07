use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::error::AppError;

/// The Agent Skills `SKILL.md`, embedded so the release binary needs no network access.
pub const SKILL_MD: &str = include_str!("assets/skills/scry/SKILL.md");

/// Skill directory name. Must match the `name` in `SKILL.md`'s frontmatter.
const SKILL_DIR: &str = "scry";
const SKILL_FILE: &str = "SKILL.md";

/// A harness whose standard user-global skill location scry knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Harness {
    /// Claude Code
    ClaudeCode,
    /// opencode
    Opencode,
    /// The open Agent Skills standard
    Agents,
}

impl Harness {
    /// The harness's standard user-global skill file path.
    pub fn global_path(self) -> PathBuf {
        match self {
            Harness::ClaudeCode => home_dir().join(".claude"),
            Harness::Opencode => config_dir().join("opencode"),
            Harness::Agents => home_dir().join(".agents"),
        }
        .join("skills")
        .join(SKILL_DIR)
        .join(SKILL_FILE)
    }
}

/// The skill file path inside a custom skills directory (`<dir>/scry/SKILL.md`).
pub fn path_in_skills_dir(dir: &Path) -> PathBuf {
    dir.join(SKILL_DIR).join(SKILL_FILE)
}

/// Write the embedded skill to `path`. Refuses to overwrite an existing file
/// unless `force` is set. Returns the path written.
pub fn install(path: &Path, force: bool) -> Result<PathBuf, AppError> {
    if path.exists() && !force {
        return Err(AppError::Usage(format!(
            "skill already installed at {}. Re-run with --force to overwrite.",
            path.display()
        )));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Internal(format!("failed to create {}: {error}", parent.display()))
        })?;
    }

    fs::write(path, SKILL_MD).map_err(|error| {
        AppError::Internal(format!("failed to write {}: {error}", path.display()))
    })?;

    Ok(path.to_path_buf())
}

/// Remove the skill installed at `path`. Errors if nothing is installed there.
/// Returns the path removed.
pub fn uninstall(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Err(AppError::Usage(format!(
            "no scry skill installed at {}.",
            path.display()
        )));
    }

    fs::remove_file(path).map_err(|error| {
        AppError::Internal(format!("failed to remove {}: {error}", path.display()))
    })?;

    // Clean up the now-empty skill directory, best effort.
    if let Some(dir) = path.parent() {
        let _ = fs::remove_dir(dir);
    }

    Ok(path.to_path_buf())
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull a `key: value` line out of the embedded frontmatter.
    fn frontmatter_value(key: &str) -> String {
        let rest = SKILL_MD
            .strip_prefix("---\n")
            .expect("SKILL.md starts with frontmatter");
        let frontmatter = rest.split("\n---\n").next().expect("frontmatter is closed");
        frontmatter
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .unwrap_or_else(|| panic!("frontmatter missing `{key}`"))
            .trim()
            .to_string()
    }

    /// Mirrors the Agent Skills `name` rule: 1-64 chars of lowercase
    /// alphanumerics separated by single hyphens.
    fn is_valid_skill_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 64
            && name.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    }

    #[test]
    fn embedded_skill_satisfies_the_standard() {
        assert_eq!(frontmatter_value("name"), SKILL_DIR);
        assert!(is_valid_skill_name(&frontmatter_value("name")));

        let description = frontmatter_value("description");
        assert!((1..=1024).contains(&description.chars().count()));
    }

    #[test]
    fn path_in_skills_dir_appends_the_skill_directory() {
        assert_eq!(
            path_in_skills_dir(Path::new(".claude/skills")),
            Path::new(".claude/skills/scry/SKILL.md")
        );
    }
}
