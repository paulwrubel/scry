use crate::position::Position;
use anstyle::AnsiColor;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::str::FromStr;
use uuid::Uuid;

pub use crate::ids::{AutomationId, NoteId, ProjectId, SectionId, TaskId};

/// Why a stored enum name could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFieldError {
    kind: &'static str,
    value: String,
    expected: &'static [&'static str],
}

impl ParseFieldError {
    fn new(kind: &'static str, value: &str, expected: &'static [&'static str]) -> Self {
        Self {
            kind,
            value: value.to_string(),
            expected,
        }
    }
}

impl Display for ParseFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid {} {:?}; expected one of: {}",
            self.kind,
            self.value,
            self.expected.join(", ")
        )
    }
}

impl std::error::Error for ParseFieldError {}

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

impl FromStr for TaskState {
    type Err = ParseFieldError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" | "Open" => Ok(Self::Open),
            "closed" | "Closed" => Ok(Self::Closed),
            _ => Err(ParseFieldError::new(
                "task state",
                value,
                &["open", "closed"],
            )),
        }
    }
}

impl TryFrom<&str> for TaskState {
    type Error = ParseFieldError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
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
        title: Option<String>,
        description: Option<String>,
        kind: Option<String>,
        state: Option<String>,
        reason: Option<String>,
        draft: Option<bool>,
        updated_at: Option<DateTime<Utc>>,
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

/// Ordered highest-first so the derived `Ord` sorts critical before elevated and normal.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Critical,
    #[serde(alias = "high")]
    Elevated,
    #[default]
    #[serde(alias = "medium")]
    #[serde(alias = "low")]
    #[serde(alias = "minimal")]
    Normal,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Elevated => "elevated",
            Self::Normal => "normal",
        }
    }
}

impl FromStr for Priority {
    type Err = ParseFieldError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "critical" | "Critical" => Ok(Self::Critical),
            "elevated" | "Elevated" => Ok(Self::Elevated),
            "normal" | "Normal" => Ok(Self::Normal),
            _ => Err(ParseFieldError::new(
                "priority",
                value,
                &["critical", "elevated", "normal"],
            )),
        }
    }
}

impl TryFrom<&str> for Priority {
    type Error = ParseFieldError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TaskSortingMode {
    #[default]
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

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
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
    pub external_identity: Option<ExternalIdentity>,
    pub external_metadata: Option<ExternalMetadata>,
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

impl FromStr for EntityType {
    type Err = ParseFieldError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "project" | "Project" => Ok(Self::Project),
            "section" | "Section" => Ok(Self::Section),
            "task" | "Task" => Ok(Self::Task),
            "note" | "Note" => Ok(Self::Note),
            "automation" | "Automation" => Ok(Self::Automation),
            _ => Err(ParseFieldError::new(
                "change entity type",
                value,
                &["project", "section", "task", "note", "automation"],
            )),
        }
    }
}

impl TryFrom<&str> for EntityType {
    type Error = ParseFieldError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
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

impl FromStr for ChangeKind {
    type Err = ParseFieldError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "create" | "Create" => Ok(Self::Create),
            "update" | "Update" => Ok(Self::Update),
            "delete" | "Delete" => Ok(Self::Delete),
            _ => Err(ParseFieldError::new(
                "change kind",
                value,
                &["create", "update", "delete"],
            )),
        }
    }
}

impl TryFrom<&str> for ChangeKind {
    type Error = ParseFieldError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
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

#[cfg(test)]
mod tests {
    use super::{ChangeKind, EntityType, Priority, TaskState};

    #[test]
    fn parses_kebab_and_variant_names() {
        assert_eq!("critical".parse::<Priority>(), Ok(Priority::Critical));
        assert_eq!("Critical".parse::<Priority>(), Ok(Priority::Critical));
        assert_eq!("elevated".parse::<Priority>(), Ok(Priority::Elevated));
        assert_eq!("Elevated".parse::<Priority>(), Ok(Priority::Elevated));
        assert_eq!("normal".parse::<Priority>(), Ok(Priority::Normal));
        assert_eq!("Normal".parse::<Priority>(), Ok(Priority::Normal));
        assert_eq!(Priority::try_from("Elevated"), Ok(Priority::Elevated));
    }

    #[test]
    fn rejects_unknown_priorities() {
        assert!("high".parse::<Priority>().is_err());
        assert!("".parse::<Priority>().is_err());
        assert!("CRITICAL".parse::<Priority>().is_err());
    }

    #[test]
    fn parses_stored_enum_names() {
        assert_eq!("open".parse::<TaskState>(), Ok(TaskState::Open));
        assert_eq!("Closed".parse::<TaskState>(), Ok(TaskState::Closed));
        assert_eq!(
            "automation".parse::<EntityType>(),
            Ok(EntityType::Automation)
        );
        assert_eq!("Task".parse::<EntityType>(), Ok(EntityType::Task));
        assert_eq!("delete".parse::<ChangeKind>(), Ok(ChangeKind::Delete));
        assert_eq!("Update".parse::<ChangeKind>(), Ok(ChangeKind::Update));
        assert_eq!(EntityType::try_from("Section"), Ok(EntityType::Section));
    }

    #[test]
    fn rejects_unknown_stored_enum_names() {
        assert!("reopened".parse::<TaskState>().is_err());
        assert!("filter".parse::<EntityType>().is_err());
        assert!("archive".parse::<ChangeKind>().is_err());
    }
}
