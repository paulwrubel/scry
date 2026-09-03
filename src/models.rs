use chrono::{DateTime, Utc};
use clap::ValueEnum;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use ratatui::{style::Style, text::Span};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::IntoEnumIterator;

pub type NoteId = i64;
pub type TaskId = i64;
pub type ProjectId = i64;
pub type StatusId = i64;

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
                .expect("StatusStyle has no skipped variants")
                .get_name(),
        )
    }
}

impl From<&str> for StatusStyle {
    fn from(value: &str) -> Self {
        Self::from_str(value, false).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tags(pub(crate) Vec<String>);

impl Tags {
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }
}

impl Display for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.iter().join(","))
    }
}

impl From<&str> for Tags {
    fn from(value: &str) -> Self {
        value.split(',').collect::<Vec<_>>().into()
    }
}

impl From<Vec<&str>> for Tags {
    fn from(value: Vec<&str>) -> Self {
        value.into_iter().map(String::from).collect_vec().into()
    }
}

impl From<Vec<String>> for Tags {
    fn from(mut value: Vec<String>) -> Self {
        value.sort();

        Self(
            value
                .into_iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .unique()
                .collect_vec(),
        )
    }
}

impl From<Tags> for Vec<String> {
    fn from(value: Tags) -> Self {
        value.0
    }
}

impl IntoIterator for Tags {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    TryFromPrimitive,
    IntoPrimitive,
    strum::Display,
    strum::EnumIter,
)]
#[repr(i64)]
pub enum Priority {
    Minimal = 5,
    Low = 4,
    #[default]
    Medium = 3,
    High = 2,
    Critical = 1,
}

impl Priority {
    pub fn color(&self) -> Color {
        match self {
            Priority::Critical => Color::Red,
            Priority::High => Color::Yellow,
            Priority::Medium => Color::Cyan,
            Priority::Low => Color::Blue,
            Priority::Minimal => Color::Magenta,
        }
    }

    pub fn next(&self) -> Self {
        Self::iter()
            .cycle()
            .skip_while(|p| p != self)
            .nth(1)
            .expect("should cycle forever")
    }

    pub fn previous(&self) -> Self {
        Self::iter()
            .rev()
            .cycle()
            .skip_while(|p| p != self)
            .nth(1)
            .expect("should cycle forever")
    }

    pub fn index(&self) -> usize {
        Self::iter().position(|p| p == *self).expect("must match")
    }

    pub fn short_span(&self) -> Span<'static> {
        Span::styled(
            format!("p{}", i64::from(*self)),
            Style::default().fg(self.color().into()),
        )
    }
}

impl From<Priority> for Span<'_> {
    fn from(value: Priority) -> Self {
        Span::styled(
            format!("p{} - {}", i64::from(value), value),
            Style::default().fg(value.color().into()),
        )
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum TaskSortingMode {
    #[default]
    #[value(aliases([
        "default",
        "alpha",
        "lex",
        "lexical",
        "lexicographical"
    ]))]
    Alphabetical,
    #[value(aliases([
        "alpha-nocase",
        "alphanocase",
        "alphaci",
    ]))]
    AlphabeticalCaseInsensitive,
    Id,
    #[value(aliases(["none","position","pos"]))]
    Manual,
    #[value(aliases(["pri"]))]
    Priority,
}

impl Display for TaskSortingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.to_possible_value()
                .expect("TaskSortingMode has no skipped variants")
                .get_name(),
        )
    }
}

impl From<&str> for TaskSortingMode {
    fn from(value: &str) -> Self {
        Self::from_str(value, false).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectTemplate {
    pub(crate) name: &'static str,
    pub(crate) entry_status_name: Option<&'static str>,
    pub(crate) task_sorting_mode: TaskSortingMode,
    pub(crate) show_priority: bool,
    pub(crate) statuses: &'static [ProjectTemplateStatus],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectTemplateStatus {
    pub(crate) name: &'static str,
    pub(crate) position: i32,
    pub(crate) color: Option<Color>,
    pub(crate) style: StatusStyle,
}

pub(crate) const PROJECT_TEMPLATES: &[ProjectTemplate] = &[
    ProjectTemplate {
        name: "todolist",
        entry_status_name: Some("todo"),
        task_sorting_mode: TaskSortingMode::Manual,
        show_priority: false,
        statuses: &[
            ProjectTemplateStatus {
                name: "todo",
                position: 0,
                color: None,
                style: StatusStyle::Unchecked,
            },
            ProjectTemplateStatus {
                name: "done",
                position: 1,
                color: Some(Color::Green),
                style: StatusStyle::Checked,
            },
        ],
    },
    ProjectTemplate {
        name: "kanban",
        entry_status_name: Some("backlog"),
        task_sorting_mode: TaskSortingMode::Priority,
        show_priority: true,
        statuses: &[
            ProjectTemplateStatus {
                name: "in-progress",
                position: 0,
                color: Some(Color::Blue),
                style: StatusStyle::None,
            },
            ProjectTemplateStatus {
                name: "backlog",
                position: 1,
                color: None,
                style: StatusStyle::None,
            },
            ProjectTemplateStatus {
                name: "done",
                position: 2,
                color: Some(Color::Green),
                style: StatusStyle::Strikethrough,
            },
        ],
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub entry_status_id: Option<StatusId>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub id: StatusId,
    pub project_id: ProjectId,
    pub name: String,
    pub position: i32,
    pub color: Option<Color>,
    pub style: StatusStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status_id: i64,
    pub position: i32,
    pub tags: Tags,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub task_id: TaskId,
    pub contents: String,
    pub created_at: DateTime<Utc>,
}
