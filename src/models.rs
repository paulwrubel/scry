use std::fmt::Display;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub type NoteID = i64;
pub type TaskID = i64;
pub type ProjectID = i64;
pub type StatusID = i64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.to_possible_value()
                .expect("Color has no skipped variants")
                .get_name(),
        )
    }
}

impl From<Color> for ratatui::style::Color {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => Self::Black,
            Color::Red => Self::Red,
            Color::Green => Self::Green,
            Color::Yellow => Self::Yellow,
            Color::Blue => Self::Blue,
            Color::Magenta => Self::Magenta,
            Color::Cyan => Self::Cyan,
            Color::Gray => Self::Gray,
            Color::DarkGray => Self::DarkGray,
            Color::LightRed => Self::LightRed,
            Color::LightGreen => Self::LightGreen,
            Color::LightYellow => Self::LightYellow,
            Color::LightBlue => Self::LightBlue,
            Color::LightMagenta => Self::LightMagenta,
            Color::LightCyan => Self::LightCyan,
            Color::White => Self::White,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum StatusStyle {
    #[default]
    #[value(alias("default"))]
    None,
    Unchecked,
    Checked,
    #[value(alias("strike"))]
    Strikethrough,
}

impl Display for StatusStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.to_possible_value()
                .expect("Style has no skipped variants")
                .get_name(),
        )
    }
}

impl From<&str> for StatusStyle {
    fn from(value: &str) -> Self {
        Self::from_str(value, false).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectID,
    pub name: String,
    pub entry_status_id: Option<StatusID>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub id: StatusID,
    pub project_id: ProjectID,
    pub name: String,
    pub position: i32,
    pub color: Option<Color>,
    pub style: StatusStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskID,
    pub project_id: ProjectID,
    pub title: String,
    pub description: Option<String>,
    pub status_id: i64,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteID,
    pub task_id: TaskID,
    pub contents: String,
    pub created_at: DateTime<Utc>,
}
