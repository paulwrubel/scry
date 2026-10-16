//! The project `README.md`, embedded at compile time so an installed binary can
//! print it without network access, mirroring how `skill.rs` embeds `SKILL.md`.

/// The `README.md`, embedded so the release binary needs no network access.
pub const README_MD: &str = include_str!("../README.md");
