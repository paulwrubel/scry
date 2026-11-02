use crate::error::StorageError;
use crate::models::{ProjectId, ProjectSnapshot};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub const SNAPSHOT_VERSION: u32 = 1;

/// The bare server-wide snapshot: every project document plus the cursor a
/// client should pull from next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub projects: Vec<ProjectSnapshot>,
    pub last_seen_seq: i64,
}

/// The importable/exportable snapshot envelope: the format version, the
/// producing app version, and one canonical project document per project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub version: u32,
    #[serde(default)]
    pub app_version: String,
    pub projects: Vec<ProjectSnapshot>,
    /// The highest changes-log `seq` reflected by this export, when the
    /// caller asked for a sync cursor. Omitted from JSON when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_seq: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportMode {
    /// Fail if any incoming project name already exists.
    Fail,
    /// Replace existing projects whose names collide with incoming ones.
    Replace,
    /// Skip incoming projects whose names already exist.
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPlan {
    pub mode: ImportMode,
    /// Projects to insert, keeping the UUID ids and foreign keys held by the snapshot file.
    pub projects: Vec<ProjectSnapshot>,
    /// Ids of existing projects to delete before inserting (Replace mode only; empty otherwise).
    pub project_ids_to_delete: Vec<ProjectId>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ImportReport {
    pub projects: usize,
    pub sections: usize,
    pub tasks: usize,
    pub notes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotFileError {
    InvalidJson(String),
    UnsupportedVersion { found: u32, supported: u32 },
}

impl fmt::Display for SnapshotFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid snapshot JSON: {message}"),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "unsupported snapshot version {found}; this build supports version {supported}"
            ),
        }
    }
}

impl std::error::Error for SnapshotFileError {}

/// Parse a snapshot file.
///
/// Only the current format version is accepted; there is no upgrade path for
/// other versions, so a mismatched document is rejected rather than guessed at.
pub fn parse_file(input: &str) -> Result<SnapshotFile, SnapshotFileError> {
    let file: SnapshotFile = serde_json::from_str(input)
        .map_err(|error| SnapshotFileError::InvalidJson(error.to_string()))?;
    if file.version != SNAPSHOT_VERSION {
        return Err(SnapshotFileError::UnsupportedVersion {
            found: file.version,
            supported: SNAPSHOT_VERSION,
        });
    }
    Ok(file)
}

/// Read a consistent server-wide snapshot.
///
/// The cursor is read before the project documents so it can never be ahead of
/// the captured state. A write that lands in between is replayed by the next
/// pull, which is idempotent; reading it afterwards could skip that write.
pub async fn build(
    store: &dyn Store,
    project: Option<ProjectId>,
) -> Result<Snapshot, StorageError> {
    let last_seen_seq = store.last_change_seq(None).await?;

    let project_ids = match project {
        Some(id) => vec![id],
        None => store
            .get_all_projects(None)
            .await?
            .into_iter()
            .map(|project| project.id)
            .collect(),
    };

    let mut projects = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        projects.push(store.get_project_snapshot(project_id, None).await?);
    }

    Ok(Snapshot {
        projects,
        last_seen_seq,
    })
}

pub async fn plan_import(
    store: &dyn Store,
    file: &SnapshotFile,
    mode: ImportMode,
) -> Result<ImportPlan, StorageError> {
    let existing = store.get_all_projects(None).await?;

    let existing_ids_by_name: HashMap<String, ProjectId> = existing
        .iter()
        .map(|project| (project.name.clone(), project.id))
        .collect();
    let existing_names: HashSet<String> = existing_ids_by_name.keys().cloned().collect();

    match mode {
        ImportMode::Fail => plan_fail(file, &existing_names),
        ImportMode::Skip => plan_skip(file, &existing_names),
        ImportMode::Replace => plan_replace(file, &existing_ids_by_name),
    }
}

fn plan_fail(
    file: &SnapshotFile,
    existing_names: &HashSet<String>,
) -> Result<ImportPlan, StorageError> {
    let mut collisions: Vec<&str> = file
        .projects
        .iter()
        .map(|entry| entry.project.name.as_str())
        .filter(|name| existing_names.contains(*name))
        .collect();

    if !collisions.is_empty() {
        collisions.sort_unstable();
        return Err(StorageError::Conflict(format!(
            "project(s) already exist: {}",
            collisions.join(", ")
        )));
    }

    Ok(ImportPlan {
        mode: ImportMode::Fail,
        projects: file.projects.clone(),
        project_ids_to_delete: Vec::new(),
    })
}

fn plan_skip(
    file: &SnapshotFile,
    existing_names: &HashSet<String>,
) -> Result<ImportPlan, StorageError> {
    let projects: Vec<ProjectSnapshot> = file
        .projects
        .iter()
        .filter(|entry| !existing_names.contains(&entry.project.name))
        .cloned()
        .collect();

    Ok(ImportPlan {
        mode: ImportMode::Skip,
        projects,
        project_ids_to_delete: Vec::new(),
    })
}

fn plan_replace(
    file: &SnapshotFile,
    existing_ids_by_name: &HashMap<String, ProjectId>,
) -> Result<ImportPlan, StorageError> {
    let project_ids_to_delete = file
        .projects
        .iter()
        .filter_map(|entry| existing_ids_by_name.get(&entry.project.name).copied())
        .collect();

    Ok(ImportPlan {
        mode: ImportMode::Replace,
        projects: file.projects.clone(),
        project_ids_to_delete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{NoteId, SectionId, TaskId};
    use crate::models::{Change, Entity, EntityType};
    use crate::models::{
        Note, Priority, Project, ProjectSnapshot, Section, Tags, Task, TaskSortingMode, TaskState,
    };
    use crate::position::Position;
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use sqlx::{PgConnection, Postgres, Transaction};
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    use uuid::Uuid;

    const PROJECT_ID: &str = "a1b2c3d4-0000-4000-8000-000000000001";
    const OTHER_PROJECT_ID: &str = "a1b2c3d4-0000-4000-8000-000000000002";
    const SECTION_ID: &str = "b1b2c3d4-0000-4000-8000-000000000003";
    const TASK_ID: &str = "c1b2c3d4-0000-4000-8000-000000000004";
    const NOTE_ID: &str = "d1b2c3d4-0000-4000-8000-000000000005";
    const EXISTING_PROJECT_ID: &str = "e1b2c3d4-0000-4000-8000-000000000006";

    fn uuid(value: &str) -> Uuid {
        Uuid::parse_str(value).expect("test uuid should parse")
    }

    fn position(value: &str) -> Position {
        Position::parse(value).expect("test position should parse")
    }

    fn project(id: &str, name: &str) -> Project {
        Project {
            id: uuid(id),
            name: name.to_string(),
            description: None,
            task_sorting_mode: TaskSortingMode::Manual,
            show_priority: false,
            created_at: Utc::now(),
        }
    }

    fn section(id: &str, project_id: ProjectId) -> Section {
        Section {
            id: uuid(id),
            project_id,
            name: "section".to_string(),
            position: position("a0"),
            color: None,
            created_at: Utc::now(),
        }
    }

    fn task(id: &str, project_id: ProjectId, section_id: Option<SectionId>) -> Task {
        Task {
            id: uuid(id),
            project_id,
            title: "task".to_string(),
            description: None,
            priority: Priority::Medium,
            section_id,
            position: position("a0"),
            state: TaskState::Open,
            tags: Tags::default(),
            created_at: Utc::now(),
        }
    }

    fn note(id: &str, task_id: TaskId) -> Note {
        Note {
            id: uuid(id),
            task_id,
            contents: "note".to_string(),
            created_at: Utc::now(),
        }
    }

    fn project_doc(id: &str, name: &str) -> ProjectSnapshot {
        ProjectSnapshot {
            project: project(id, name),
            sections: vec![],
            tasks: vec![],
            notes: vec![],
        }
    }

    fn full_project_doc(id: &str, name: &str) -> ProjectSnapshot {
        let project = project(id, name);
        let section = section(SECTION_ID, project.id);
        let task = task(TASK_ID, project.id, Some(section.id));
        let note = note(NOTE_ID, task.id);

        ProjectSnapshot {
            project,
            sections: vec![section],
            tasks: vec![task],
            notes: vec![note],
        }
    }

    fn snapshot_file(projects: Vec<ProjectSnapshot>) -> SnapshotFile {
        SnapshotFile {
            version: SNAPSHOT_VERSION,
            app_version: String::new(),
            projects,
            last_seen_seq: None,
        }
    }

    // scry-core has no async runtime dependency, and the fake store's futures
    // resolve immediately, so polling once in a loop is enough.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    struct EmptyStore;

    #[async_trait::async_trait]
    impl Store for EmptyStore {
        async fn get_all_projects(
            &self,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Project>, StorageError> {
            Ok(Vec::new())
        }

        async fn create_task(
            &self,
            _: Task,
            _: Option<&mut PgConnection>,
        ) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_task_by_id(
            &self,
            _: TaskId,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn find_task_ids_by_handle(
            &self,
            _: &crate::ids::Handle,
            _: i64,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<TaskId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_task_ids(
            &self,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<TaskId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_tasks_by_project_id(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_tasks_by_section_id(
            &self,
            _: SectionId,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_task(
            &self,
            _: Task,
            _: Option<&mut PgConnection>,
        ) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_task(
            &self,
            _: TaskId,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_note(
            &self,
            _: Note,
            _: Option<&mut PgConnection>,
        ) -> Result<Note, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_note_by_id(
            &self,
            _: NoteId,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_notes_by_task_id(
            &self,
            _: TaskId,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_notes_by_project_id(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_note(
            &self,
            _: Note,
            _: Option<&mut PgConnection>,
        ) -> Result<Note, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_note(
            &self,
            _: NoteId,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn find_note_ids_by_handle(
            &self,
            _: &crate::ids::Handle,
            _: i64,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<NoteId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_note_ids(
            &self,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<NoteId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_section(
            &self,
            _: Section,
            _: Option<&mut PgConnection>,
        ) -> Result<Section, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_section_by_id(
            &self,
            _: SectionId,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_section_by_name(
            &self,
            _: ProjectId,
            _: &str,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_sections_by_project_id(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_section(
            &self,
            _: Section,
            _: Option<&mut PgConnection>,
        ) -> Result<Section, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_section(
            &self,
            _: SectionId,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_project(
            &self,
            _: Project,
            _: Option<&mut PgConnection>,
        ) -> Result<Project, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_by_id(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Project>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_snapshot(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<ProjectSnapshot, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_by_name(
            &self,
            _: &str,
            _: Option<&mut PgConnection>,
        ) -> Result<Option<Project>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_project(
            &self,
            _: Project,
            _: Option<&mut PgConnection>,
        ) -> Result<Project, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_project_by_id(
            &self,
            _: ProjectId,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn import(
            &self,
            _: ImportPlan,
            _: Option<&mut PgConnection>,
        ) -> Result<ImportReport, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn is_operation_applied(
            &self,
            _: Uuid,
            _: Option<&mut PgConnection>,
        ) -> Result<bool, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn record_applied_operation(
            &self,
            _: Uuid,
            _: DateTime<Utc>,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn record_change(
            &self,
            _: Change,
            _: Option<&mut PgConnection>,
        ) -> Result<i64, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn changes_after(
            &self,
            _: i64,
            _: Option<&mut PgConnection>,
        ) -> Result<Vec<Change>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn last_change_seq(&self, _: Option<&mut PgConnection>) -> Result<i64, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn is_tombstoned(
            &self,
            _: EntityType,
            _: Uuid,
            _: Option<&mut PgConnection>,
        ) -> Result<bool, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_tombstone(
            &self,
            _: Entity,
            _: DateTime<Utc>,
            _: Option<&mut PgConnection>,
        ) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn begin_apply(&self) -> Result<Transaction<'static, Postgres>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }
    }

    #[test]
    fn plan_fail_rejects_colliding_names() {
        let file = snapshot_file(vec![
            project_doc(PROJECT_ID, "keep"),
            project_doc(OTHER_PROJECT_ID, "collide"),
        ]);
        let existing = HashSet::from(["collide".to_string()]);

        assert!(matches!(
            plan_fail(&file, &existing),
            Err(StorageError::Conflict(_))
        ));
    }

    #[test]
    fn plan_fail_preserves_every_id() {
        let file = snapshot_file(vec![full_project_doc(PROJECT_ID, "alpha")]);

        let plan =
            plan_fail(&file, &HashSet::new()).expect("fail should succeed when no name collides");

        assert_eq!(plan.mode, ImportMode::Fail);
        assert_eq!(plan.projects.len(), 1);
        assert!(plan.project_ids_to_delete.is_empty());

        let imported = &plan.projects[0];
        assert_eq!(imported.project.id, uuid(PROJECT_ID));
        assert_eq!(imported.sections[0].id, uuid(SECTION_ID));
        assert_eq!(imported.sections[0].project_id, imported.project.id);
        assert_eq!(imported.tasks[0].id, uuid(TASK_ID));
        assert_eq!(imported.tasks[0].project_id, imported.project.id);
        assert_eq!(imported.tasks[0].section_id, Some(imported.sections[0].id));
        assert_eq!(imported.notes[0].id, uuid(NOTE_ID));
        assert_eq!(imported.notes[0].task_id, imported.tasks[0].id);
    }

    #[test]
    fn plan_skip_drops_colliding_projects_and_preserves_other_ids() {
        let file = snapshot_file(vec![
            project_doc(PROJECT_ID, "keep"),
            full_project_doc(OTHER_PROJECT_ID, "skip"),
        ]);
        let existing = HashSet::from(["skip".to_string()]);

        let plan = plan_skip(&file, &existing).expect("skip should succeed");

        assert_eq!(plan.mode, ImportMode::Skip);
        assert_eq!(plan.projects.len(), 1);
        assert_eq!(plan.projects[0].project.name, "keep");
        assert_eq!(plan.projects[0].project.id, uuid(PROJECT_ID));
        assert!(plan.project_ids_to_delete.is_empty());
    }

    #[test]
    fn plan_replace_lists_collisions_and_preserves_incoming_ids() {
        let file = snapshot_file(vec![
            full_project_doc(PROJECT_ID, "alpha"),
            project_doc(OTHER_PROJECT_ID, "beta"),
        ]);
        let existing_id = uuid(EXISTING_PROJECT_ID);
        let existing = HashMap::from([("alpha".to_string(), existing_id)]);

        let plan = plan_replace(&file, &existing).expect("replace should succeed");

        assert_eq!(plan.mode, ImportMode::Replace);
        assert_eq!(plan.projects.len(), 2);
        assert_eq!(plan.projects[0].project.id, uuid(PROJECT_ID));
        assert_eq!(plan.projects[1].project.id, uuid(OTHER_PROJECT_ID));
        assert_eq!(plan.project_ids_to_delete, vec![existing_id]);
    }

    #[test]
    fn plan_import_keeps_a_full_snapshot_file_unchanged() {
        let file = snapshot_file(vec![full_project_doc(PROJECT_ID, "alpha")]);

        let plan = block_on(plan_import(&EmptyStore, &file, ImportMode::Fail))
            .expect("import planning should succeed");

        assert_eq!(plan.mode, ImportMode::Fail);
        assert!(plan.project_ids_to_delete.is_empty());
        assert_eq!(plan.projects.len(), file.projects.len());

        let imported = &plan.projects[0];
        assert_eq!(imported.project.id, uuid(PROJECT_ID));
        assert_eq!(imported.sections[0].id, uuid(SECTION_ID));
        assert_eq!(imported.sections[0].project_id, imported.project.id);
        assert_eq!(imported.tasks[0].id, uuid(TASK_ID));
        assert_eq!(imported.tasks[0].project_id, imported.project.id);
        assert_eq!(imported.tasks[0].section_id, Some(imported.sections[0].id));
        assert_eq!(imported.notes[0].id, uuid(NOTE_ID));
        assert_eq!(imported.notes[0].task_id, imported.tasks[0].id);

        assert_eq!(
            serde_json::to_value(&plan.projects).unwrap(),
            serde_json::to_value(&file.projects).unwrap(),
            "planning must not rewrite ids, foreign keys, or timestamps"
        );
    }

    #[test]
    fn parse_rejects_unsupported_version() {
        let v4 = r#"{ "version": 4, "projects": [] }"#;

        assert!(matches!(
            parse_file(v4),
            Err(SnapshotFileError::UnsupportedVersion {
                found: 4,
                supported: SNAPSHOT_VERSION
            })
        ));
    }

    #[test]
    fn snapshot_file_round_trips_string_positions() {
        let mut original = snapshot_file(vec![full_project_doc(PROJECT_ID, "alpha")]);
        original.projects[0].sections[0].position = position("a2");
        original.projects[0].tasks[0].position = position("a5");

        let serialized = serde_json::to_string(&original).expect("snapshot file should serialize");
        let parsed = parse_file(&serialized).expect("current version should parse");

        assert_eq!(parsed.version, SNAPSHOT_VERSION);
        assert_eq!(parsed.projects[0].sections[0].position, position("a2"));
        assert_eq!(parsed.projects[0].tasks[0].position, position("a5"));
    }

    #[test]
    fn snapshot_file_omits_the_cursor_by_default() {
        let serialized =
            serde_json::to_value(snapshot_file(vec![])).expect("snapshot file should serialize");

        assert!(serialized.get("last_seen_seq").is_none());
    }

    #[test]
    fn snapshot_file_carries_the_cursor_when_set() {
        let mut file = snapshot_file(vec![]);
        file.last_seen_seq = Some(7);

        let serialized = serde_json::to_value(&file).expect("snapshot file should serialize");

        assert_eq!(serialized["last_seen_seq"], json!(7));
    }

    #[test]
    fn parse_file_accepts_snapshots_with_and_without_a_cursor() {
        let without_cursor = format!(r#"{{ "version": {SNAPSHOT_VERSION}, "projects": [] }}"#);
        let with_cursor =
            format!(r#"{{ "version": {SNAPSHOT_VERSION}, "projects": [], "last_seen_seq": 7 }}"#);

        let parsed_without =
            parse_file(&without_cursor).expect("snapshot file without a cursor should parse");
        let parsed_with =
            parse_file(&with_cursor).expect("snapshot file with a cursor should parse");

        assert_eq!(parsed_without.last_seen_seq, None);
        assert_eq!(parsed_with.last_seen_seq, Some(7));
        assert_eq!(parsed_without.version, SNAPSHOT_VERSION);
        assert_eq!(parsed_with.version, SNAPSHOT_VERSION);
    }
}
