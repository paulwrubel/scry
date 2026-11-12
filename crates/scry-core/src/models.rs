use crate::position::Position;
use anstyle::AnsiColor;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

pub use crate::ids::{AutomationId, NoteId, ProjectId, SectionId, TaskId};

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
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

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    #[default]
    Open,
    Closed,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

/// Identifies a GitHub issue or pull request that a task tracks.
///
/// It deliberately does not record whether the number is an issue or a pull,
/// because the number namespace is shared and the kind can change server-side.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ExternalIdentity {
    Github {
        owner: String,
        repo: String,
        number: u64,
    },
}

/// The most recent GitHub metadata fetched for an [`ExternalIdentity`].
///
/// `kind` is `"issue"` or `"pull"` and `state` is `"open"`, `"closed"`, or
/// `"merged"`; both are kept as plain strings because GitHub owns those
/// vocabularies and new values must not break deserialization.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ExternalMetadata {
    Github {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        state: Option<String>,
        #[serde(default)]
        reason: Option<String>,
        #[serde(default)]
        draft: Option<bool>,
        #[serde(default)]
        updated_at: Option<DateTime<Utc>>,
        #[serde(default)]
        error: Option<String>,
    },
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// Creation order, oldest first. `id` is kept as an alias for old configs.
    #[value(alias = "id")]
    Created,
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
    pub position: &'static str,
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
                position: "a0",
                color: None,
            },
            ProjectTemplateSection {
                name: "in-progress",
                position: "a1",
                color: Some(Color::Blue),
            },
            ProjectTemplateSection {
                name: "review",
                position: "a2",
                color: Some(Color::Yellow),
            },
        ],
    },
];

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub task_sorting_mode: TaskSortingMode,
    pub show_priority: bool,
    pub created_at: DateTime<Utc>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: SectionId,
    pub project_id: ProjectId,
    pub name: String,
    pub position: Position,
    pub color: Option<Color>,
    pub created_at: DateTime<Utc>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub section_id: Option<SectionId>,
    pub position: Position,
    pub state: TaskState,
    pub tags: Tags,
    pub created_at: DateTime<Utc>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub task_id: TaskId,
    pub contents: String,
    pub created_at: DateTime<Utc>,
}

pub const MAX_AUTOMATION_ACTIONS: usize = 1;

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationTrigger {
    TaskCreated,
    TaskClosed,
    TaskOpened,
    TaskEntersSection { section_id: SectionId },
}

impl AutomationTrigger {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::TaskCreated => "task_created",
            Self::TaskClosed => "task_closed",
            Self::TaskOpened => "task_opened",
            Self::TaskEntersSection { .. } => "task_enters_section",
        }
    }

    pub fn section_id(&self) -> Option<SectionId> {
        match self {
            Self::TaskEntersSection { section_id } => Some(*section_id),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationAction {
    MoveToSection { section_id: Option<SectionId> },
    CloseTask,
    OpenTask,
    SetPriority { priority: Priority },
}

impl AutomationAction {
    pub fn section_id(&self) -> Option<SectionId> {
        match self {
            Self::MoveToSection { section_id } => *section_id,
            _ => None,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Automation {
    pub id: AutomationId,
    pub project_id: ProjectId,
    pub trigger: AutomationTrigger,
    pub actions: Vec<AutomationAction>,
    pub created_at: DateTime<Utc>,
}

impl Automation {
    /// Whether this rule's trigger or any action refers to the section.
    pub fn references_section(&self, section_id: SectionId) -> bool {
        self.trigger.section_id() == Some(section_id)
            || self
                .actions
                .iter()
                .any(|action| action.section_id() == Some(section_id))
    }
}

/// A JSON object holding the model field names and the values they take after
/// the operation, as recorded in the changes log.
pub type FieldValues = serde_json::Value;

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Project,
    Section,
    Task,
    Note,
    Automation,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Section => "section",
            Self::Task => "task",
            Self::Note => "note",
            Self::Automation => "automation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    Project(ProjectId),
    Section(SectionId),
    Task(TaskId),
    Note(NoteId),
    Automation(AutomationId),
}

impl Entity {
    pub fn entity_type(&self) -> EntityType {
        match self {
            Self::Project(_) => EntityType::Project,
            Self::Section(_) => EntityType::Section,
            Self::Task(_) => EntityType::Task,
            Self::Note(_) => EntityType::Note,
            Self::Automation(_) => EntityType::Automation,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            Self::Project(id)
            | Self::Section(id)
            | Self::Task(id)
            | Self::Note(id)
            | Self::Automation(id) => *id,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Create,
    Update,
    Delete,
}

impl ChangeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

/// One row of the changes log, as written by the server.
///
/// `seq` is assigned by the database and defines the row's order. Writers pass
/// a placeholder (`0`) to [`Store::record_change`](crate::store::Store::record_change),
/// which ignores it and returns the assigned value; readers get the real value
/// back from the pull query.
///
/// `created_at` is when the server wrote the row and is supplied by the
/// service. `client_timestamp` is recorded as the client reported it and never
/// compared; `seq` remains the only ordering.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub seq: i64,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub kind: ChangeKind,
    pub operation_id: Uuid,
    pub client_timestamp: Option<DateTime<Utc>>,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub field_values: FieldValues,
    pub created_at: DateTime<Utc>,
}

/// A project together with all of its sections, tasks, notes, and automations.
///
/// This is scry's canonical project document: `GET /projects/{id}/snapshot`
/// returns it directly, and each entry in a snapshot file is the same
/// object. The project's own fields are flattened into the object's top
/// level, and all ids and positions are carried verbatim so the document can
/// be serialized and deserialized without loss.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    #[serde(flatten)]
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub notes: Vec<Note>,
    #[serde(default)]
    pub automations: Vec<Automation>,
}
