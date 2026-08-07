use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type TaskID = i64;
pub type ProjectID = i64;

/// Canonical list of all supported state colors: (name, ratatui Color).
pub const STATE_COLORS: &[(&str, ratatui::style::Color)] = &[
    ("Red", ratatui::style::Color::Red),
    ("Green", ratatui::style::Color::Green),
    ("Yellow", ratatui::style::Color::Yellow),
    ("Blue", ratatui::style::Color::Blue),
    ("Magenta", ratatui::style::Color::Magenta),
    ("Cyan", ratatui::style::Color::Cyan),
    ("Gray", ratatui::style::Color::Gray),
    ("DarkGray", ratatui::style::Color::DarkGray),
    ("LightRed", ratatui::style::Color::LightRed),
    ("LightGreen", ratatui::style::Color::LightGreen),
    ("LightYellow", ratatui::style::Color::LightYellow),
    ("LightBlue", ratatui::style::Color::LightBlue),
    ("LightMagenta", ratatui::style::Color::LightMagenta),
    ("LightCyan", ratatui::style::Color::LightCyan),
    ("White", ratatui::style::Color::White),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color(pub String);

impl From<Color> for ratatui::style::Color {
    fn from(c: Color) -> Self {
        STATE_COLORS
            .iter()
            .find(|(name, _)| *name == c.0.as_str())
            .map(|(_, color)| *color)
            .unwrap_or(Self::Reset)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectID,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: i64,
    pub project_id: ProjectID,
    pub name: String,
    pub position: i32,
    pub is_completed: bool,
    pub is_entry: bool,
    pub color: Option<Color>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskID,
    pub project_id: ProjectID,
    pub title: String,
    pub description: Option<String>,
    pub state_id: i64,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
