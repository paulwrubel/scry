use anstyle::AnsiColor;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub type NoteId = i64;
pub type TaskId = i64;
pub type ProjectId = i64;
pub type SectionId = i64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
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

impl From<Color> for AnsiColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => Self::Black,
            Color::Red => Self::Red,
            Color::Green => Self::Green,
            Color::Yellow => Self::Yellow,
            Color::Blue => Self::Blue,
            Color::Magenta => Self::Magenta,
            Color::Cyan => Self::Cyan,
            Color::Gray => Self::White,
            Color::DarkGray => Self::BrightBlack,
            Color::LightRed => Self::BrightRed,
            Color::LightGreen => Self::BrightGreen,
            Color::LightYellow => Self::BrightYellow,
            Color::LightBlue => Self::BrightBlue,
            Color::LightMagenta => Self::BrightMagenta,
            Color::LightCyan => Self::BrightCyan,
            Color::White => Self::BrightWhite,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum CloseReason {
    Completed,
    Cancelled,
}

impl CloseReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl Display for CloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Lifecycle {
    #[default]
    Open,
    Closed {
        closed_at: DateTime<Utc>,
        reason: CloseReason,
    },
}

impl Lifecycle {
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    pub fn closed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Open => None,
            Self::Closed { closed_at, .. } => Some(*closed_at),
        }
    }

    pub fn reason(&self) -> Option<CloseReason> {
        match self {
            Self::Open => None,
            Self::Closed { reason, .. } => Some(*reason),
        }
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
    ValueEnum,
    TryFromPrimitive,
    IntoPrimitive,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[repr(i64)]
pub enum Priority {
    #[value(alias("p5"))]
    Minimal = 5,
    #[value(alias("p4"))]
    Low = 4,
    #[default]
    #[value(alias("p3"))]
    Medium = 3,
    #[value(alias("p2"))]
    High = 2,
    #[value(alias("p1"))]
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
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
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
pub struct ProjectTemplate {
    pub name: &'static str,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
    pub sections: &'static [ProjectTemplateSection],
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectTemplateSection {
    pub name: &'static str,
    pub position: i32,
    pub color: Option<Color>,
}

pub const PROJECT_TEMPLATES: &[ProjectTemplate] = &[
    ProjectTemplate {
        name: "todolist",
        task_sorting_mode: TaskSortingMode::Manual,
        show_priority: false,
        sections: &[],
    },
    ProjectTemplate {
        name: "kanban",
        task_sorting_mode: TaskSortingMode::Priority,
        show_priority: true,
        sections: &[
            ProjectTemplateSection {
                name: "backlog",
                position: 0,
                color: None,
            },
            ProjectTemplateSection {
                name: "in-progress",
                position: 1,
                color: Some(Color::Blue),
            },
            ProjectTemplateSection {
                name: "review",
                position: 2,
                color: Some(Color::Yellow),
            },
        ],
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: SectionId,
    pub project_id: ProjectId,
    pub name: String,
    pub position: i32,
    pub color: Option<Color>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub section_id: Option<SectionId>,
    pub position: i32,
    pub lifecycle: Lifecycle,
    pub tags: Tags,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub fn is_closed(&self) -> bool {
        self.lifecycle.is_closed()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub task_id: TaskId,
    pub contents: String,
    pub created_at: DateTime<Utc>,
}
