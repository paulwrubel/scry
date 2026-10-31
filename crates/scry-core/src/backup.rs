use crate::error::StorageError;
use crate::models::{Note, Project, ProjectId, Section, Task};
use crate::position::{Position, n_keys_between};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

pub const BACKUP_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub version: u32,
    #[serde(default)]
    pub app_version: String,
    pub projects: Vec<BackupProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProject {
    // flatten keeps each project's own fields at the object's top level, alongside
    // the related sections, tasks, and notes, so the format reads naturally.
    #[serde(flatten)]
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub notes: Vec<Note>,
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
    /// Projects to insert, keeping the UUID ids and foreign keys held by the backup.
    pub projects: Vec<BackupProject>,
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
pub enum BackupError {
    InvalidJson(String),
    UnsupportedVersion { found: u32, supported: u32 },
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid backup JSON: {message}"),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "unsupported backup version {found}; this build supports version {supported}"
            ),
        }
    }
}

impl std::error::Error for BackupError {}

/// Parse a backup document.
///
/// Version 4 is the current format and is deserialized directly. Version 3
/// backups carry UUID identity but store section and task positions as integers;
/// they are upgraded in place to fractional position keys. Versions 1 and 2,
/// written before UUID identity, are rejected rather than upgraded, so they must
/// be transformed manually before import.
pub fn parse(input: &str) -> Result<Backup, BackupError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| BackupError::InvalidJson(error.to_string()))?;

    let version = value
        .get("version")
        .ok_or_else(|| {
            BackupError::InvalidJson("backup is missing a numeric version field".to_string())
        })?
        .as_u64()
        .ok_or_else(|| {
            BackupError::InvalidJson("backup version field must be a positive integer".to_string())
        })?;
    let version = u32::try_from(version).map_err(|_| {
        BackupError::InvalidJson(format!("backup version {version} is out of range"))
    })?;

    match version {
        BACKUP_VERSION => serde_json::from_value(value)
            .map_err(|error| BackupError::InvalidJson(error.to_string())),
        3 => upgrade_v3(value),
        _ => Err(BackupError::UnsupportedVersion {
            found: version,
            supported: BACKUP_VERSION,
        }),
    }
}

fn upgrade_v3(mut value: serde_json::Value) -> Result<Backup, BackupError> {
    let projects = value
        .get_mut("projects")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            BackupError::InvalidJson("backup is missing a projects array".to_string())
        })?;

    for project in projects {
        upgrade_v3_section_positions(project)?;
        upgrade_v3_task_positions(project)?;
    }

    value["version"] = serde_json::Value::from(4);
    serde_json::from_value(value).map_err(|error| BackupError::InvalidJson(error.to_string()))
}

fn upgrade_v3_section_positions(project: &mut serde_json::Value) -> Result<(), BackupError> {
    let Some(sections) = project
        .get_mut("sections")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Ok(());
    };

    let ordered = order_v3_entities(sections, 0..sections.len())?;
    let keys = n_keys_between(None, None, ordered.len())
        .map_err(|error| BackupError::InvalidJson(error.to_string()))?;

    for (key, index) in keys.into_iter().zip(ordered) {
        set_position(&mut sections[index], key);
    }
    Ok(())
}

fn upgrade_v3_task_positions(project: &mut serde_json::Value) -> Result<(), BackupError> {
    let Some(tasks) = project
        .get_mut("tasks")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Ok(());
    };

    let mut groups: BTreeMap<Option<String>, Vec<usize>> = BTreeMap::new();
    for (index, task) in tasks.iter().enumerate() {
        let group = match task.get("section_id") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(id)) => Some(id.clone()),
            _ => {
                return Err(BackupError::InvalidJson(
                    "version 3 task is missing a section_id".to_string(),
                ));
            }
        };
        groups.entry(group).or_default().push(index);
    }

    for indices in groups.into_values() {
        let ordered = order_v3_entities(tasks, indices)?;
        let keys = n_keys_between(None, None, ordered.len())
            .map_err(|error| BackupError::InvalidJson(error.to_string()))?;

        for (key, index) in keys.into_iter().zip(ordered) {
            set_position(&mut tasks[index], key);
        }
    }
    Ok(())
}

fn order_v3_entities(
    entities: &[serde_json::Value],
    indices: impl IntoIterator<Item = usize>,
) -> Result<Vec<usize>, BackupError> {
    let mut ordered = indices
        .into_iter()
        .map(|index| {
            let entity = &entities[index];
            let position = entity
                .get("position")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| {
                    BackupError::InvalidJson(
                        "version 3 entity is missing an integer position".to_string(),
                    )
                })?;
            let id = entity
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    BackupError::InvalidJson("version 3 entity is missing a string id".to_string())
                })?;
            Ok((position, id.to_string(), index))
        })
        .collect::<Result<Vec<_>, BackupError>>()?;

    ordered.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(ordered.into_iter().map(|(_, _, index)| index).collect())
}

fn set_position(entity: &mut serde_json::Value, position: Position) {
    if let Some(object) = entity.as_object_mut() {
        object.insert(
            "position".to_string(),
            serde_json::Value::String(position.to_string()),
        );
    }
}

pub async fn export(store: &dyn Store, project: Option<ProjectId>) -> Result<Backup, StorageError> {
    let projects = match project {
        Some(id) => store
            .get_project_by_id(id)
            .await?
            .into_iter()
            .collect::<Vec<_>>(),
        None => store.get_all_projects().await?,
    };

    let mut backup_projects = Vec::with_capacity(projects.len());
    for project in projects {
        let sections = store.get_all_sections_by_project_id(project.id).await?;
        let tasks = store.get_all_tasks_by_project_id(project.id).await?;
        let notes = store.get_all_notes_by_project_id(project.id).await?;
        backup_projects.push(BackupProject {
            project,
            sections,
            tasks,
            notes,
        });
    }

    Ok(Backup {
        version: BACKUP_VERSION,
        // the binary crate stamps the real value when it writes a backup out
        app_version: String::new(),
        projects: backup_projects,
    })
}

pub async fn plan_import(
    store: &dyn Store,
    backup: &Backup,
    mode: ImportMode,
) -> Result<ImportPlan, StorageError> {
    let existing = store.get_all_projects().await?;

    let existing_ids_by_name: HashMap<String, ProjectId> = existing
        .iter()
        .map(|project| (project.name.clone(), project.id))
        .collect();
    let existing_names: HashSet<String> = existing_ids_by_name.keys().cloned().collect();

    match mode {
        ImportMode::Fail => plan_fail(backup, &existing_names),
        ImportMode::Skip => plan_skip(backup, &existing_names),
        ImportMode::Replace => plan_replace(backup, &existing_ids_by_name),
    }
}

fn plan_fail(
    backup: &Backup,
    existing_names: &HashSet<String>,
) -> Result<ImportPlan, StorageError> {
    let mut collisions: Vec<&str> = backup
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
        projects: backup.projects.clone(),
        project_ids_to_delete: Vec::new(),
    })
}

fn plan_skip(
    backup: &Backup,
    existing_names: &HashSet<String>,
) -> Result<ImportPlan, StorageError> {
    let projects: Vec<BackupProject> = backup
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
    backup: &Backup,
    existing_ids_by_name: &HashMap<String, ProjectId>,
) -> Result<ImportPlan, StorageError> {
    let project_ids_to_delete = backup
        .projects
        .iter()
        .filter_map(|entry| existing_ids_by_name.get(&entry.project.name).copied())
        .collect();

    Ok(ImportPlan {
        mode: ImportMode::Replace,
        projects: backup.projects.clone(),
        project_ids_to_delete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{NoteId, SectionId, TaskId};
    use crate::models::{Color, Lifecycle, Priority, Tags, TaskSortingMode};
    use crate::store::{ProjectSnapshot, TaskContentUpdate, TaskToCreate};
    use chrono::Utc;
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
            lifecycle: Lifecycle::Open,
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

    fn backup_project(id: &str, name: &str) -> BackupProject {
        BackupProject {
            project: project(id, name),
            sections: vec![],
            tasks: vec![],
            notes: vec![],
        }
    }

    fn full_backup_project(id: &str, name: &str) -> BackupProject {
        let project = project(id, name);
        let section = section(SECTION_ID, project.id);
        let task = task(TASK_ID, project.id, Some(section.id));
        let note = note(NOTE_ID, task.id);

        BackupProject {
            project,
            sections: vec![section],
            tasks: vec![task],
            notes: vec![note],
        }
    }

    fn backup(projects: Vec<BackupProject>) -> Backup {
        Backup {
            version: BACKUP_VERSION,
            app_version: String::new(),
            projects,
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
        async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError> {
            Ok(Vec::new())
        }

        async fn create_task(&self, _: TaskToCreate) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_task_by_id(&self, _: TaskId) -> Result<Option<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn find_task_ids_by_short_id(
            &self,
            _: &crate::ids::ShortId,
            _: i64,
        ) -> Result<Vec<TaskId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_task_ids(&self) -> Result<Vec<TaskId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_tasks_by_project_id(
            &self,
            _: ProjectId,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_tasks_by_section_id(
            &self,
            _: SectionId,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_task(&self, _: TaskId, _: TaskContentUpdate) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn move_task(
            &self,
            _: TaskId,
            _: Option<SectionId>,
            _: Position,
        ) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn set_lifecycle(&self, _: TaskId, _: Lifecycle) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn duplicate_task(&self, _: TaskId, _: TaskId) -> Result<Task, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_task(&self, _: TaskId) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_note(&self, _: NoteId, _: TaskId, _: String) -> Result<Note, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_note_by_id(&self, _: NoteId) -> Result<Option<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_notes_by_task_id(&self, _: TaskId) -> Result<Vec<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_notes_by_project_id(
            &self,
            _: ProjectId,
        ) -> Result<Vec<Note>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_note(&self, _: Note) -> Result<Note, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_note(&self, _: NoteId) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn find_note_ids_by_short_id(
            &self,
            _: &crate::ids::ShortId,
            _: i64,
        ) -> Result<Vec<NoteId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_note_ids(&self) -> Result<Vec<NoteId>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_section(
            &self,
            _: SectionId,
            _: ProjectId,
            _: String,
            _: Position,
            _: Option<Color>,
        ) -> Result<Section, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_section_by_id(&self, _: SectionId) -> Result<Option<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_section_by_name(
            &self,
            _: ProjectId,
            _: &str,
        ) -> Result<Option<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_all_sections_by_project_id(
            &self,
            _: ProjectId,
        ) -> Result<Vec<Section>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_section(&self, _: Section) -> Result<Section, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_section(&self, _: SectionId) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn create_project(
            &self,
            _: ProjectId,
            _: String,
            _: Option<String>,
            _: TaskSortingMode,
            _: bool,
        ) -> Result<Project, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_by_id(&self, _: ProjectId) -> Result<Option<Project>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_snapshot(
            &self,
            _: ProjectId,
        ) -> Result<ProjectSnapshot, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn get_project_by_name(&self, _: &str) -> Result<Option<Project>, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn update_project(&self, _: Project) -> Result<Project, StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn delete_project_by_id(&self, _: ProjectId) -> Result<(), StorageError> {
            unimplemented!("plan_import only reads projects")
        }

        async fn import(&self, _: ImportPlan) -> Result<ImportReport, StorageError> {
            unimplemented!("plan_import only reads projects")
        }
    }

    #[test]
    fn plan_fail_rejects_colliding_names() {
        let backup = backup(vec![
            backup_project(PROJECT_ID, "keep"),
            backup_project(OTHER_PROJECT_ID, "collide"),
        ]);
        let existing = HashSet::from(["collide".to_string()]);

        assert!(matches!(
            plan_fail(&backup, &existing),
            Err(StorageError::Conflict(_))
        ));
    }

    #[test]
    fn plan_fail_preserves_every_id() {
        let backup = backup(vec![full_backup_project(PROJECT_ID, "alpha")]);

        let plan =
            plan_fail(&backup, &HashSet::new()).expect("fail should succeed when no name collides");

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
        let backup = backup(vec![
            backup_project(PROJECT_ID, "keep"),
            full_backup_project(OTHER_PROJECT_ID, "skip"),
        ]);
        let existing = HashSet::from(["skip".to_string()]);

        let plan = plan_skip(&backup, &existing).expect("skip should succeed");

        assert_eq!(plan.mode, ImportMode::Skip);
        assert_eq!(plan.projects.len(), 1);
        assert_eq!(plan.projects[0].project.name, "keep");
        assert_eq!(plan.projects[0].project.id, uuid(PROJECT_ID));
        assert!(plan.project_ids_to_delete.is_empty());
    }

    #[test]
    fn plan_replace_lists_collisions_and_preserves_incoming_ids() {
        let backup = backup(vec![
            full_backup_project(PROJECT_ID, "alpha"),
            backup_project(OTHER_PROJECT_ID, "beta"),
        ]);
        let existing_id = uuid(EXISTING_PROJECT_ID);
        let existing = HashMap::from([("alpha".to_string(), existing_id)]);

        let plan = plan_replace(&backup, &existing).expect("replace should succeed");

        assert_eq!(plan.mode, ImportMode::Replace);
        assert_eq!(plan.projects.len(), 2);
        assert_eq!(plan.projects[0].project.id, uuid(PROJECT_ID));
        assert_eq!(plan.projects[1].project.id, uuid(OTHER_PROJECT_ID));
        assert_eq!(plan.project_ids_to_delete, vec![existing_id]);
    }

    #[test]
    fn plan_import_keeps_a_full_backup_unchanged() {
        let backup = backup(vec![full_backup_project(PROJECT_ID, "alpha")]);

        let plan = block_on(plan_import(&EmptyStore, &backup, ImportMode::Fail))
            .expect("import planning should succeed");

        assert_eq!(plan.mode, ImportMode::Fail);
        assert!(plan.project_ids_to_delete.is_empty());
        assert_eq!(plan.projects.len(), backup.projects.len());

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
            serde_json::to_value(&backup.projects).unwrap(),
            "planning must not rewrite ids, foreign keys, or timestamps"
        );
    }

    #[test]
    fn parse_rejects_unsupported_version() {
        let v1 = r#"{ "version": 1, "projects": [] }"#;

        assert!(matches!(
            parse(v1),
            Err(BackupError::UnsupportedVersion {
                found: 1,
                supported: BACKUP_VERSION
            })
        ));

        let v2 = r#"{
            "version": 2,
            "projects": [
                {
                    "id": 1,
                    "name": "legacy",
                    "description": null,
                    "task_sorting_mode": "manual",
                    "show_priority": false,
                    "created_at": "2026-01-01T00:00:00Z",
                    "sections": [],
                    "tasks": [],
                    "notes": []
                }
            ]
        }"#;

        assert!(matches!(
            parse(v2),
            Err(BackupError::UnsupportedVersion { found: 2, .. })
        ));
    }

    #[test]
    fn v3_backup_upgrades_positions_in_recorded_order() {
        const SECTION_ONE: &str = "b1b2c3d4-0000-4000-8000-000000000001";
        const SECTION_TWO: &str = "b1b2c3d4-0000-4000-8000-000000000002";

        let v3 = r#"{
            "version": 3,
            "app_version": "0.0.0",
            "projects": [
                {
                    "id": "a1b2c3d4-0000-4000-8000-000000000001",
                    "name": "legacy",
                    "description": null,
                    "task_sorting_mode": "manual",
                    "show_priority": false,
                    "created_at": "2026-01-01T00:00:00Z",
                    "sections": [
                        {
                            "id": "b1b2c3d4-0000-4000-8000-000000000005",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "name": "five",
                            "position": 5,
                            "color": null,
                            "created_at": "2026-01-01T00:00:00Z"
                        },
                        {
                            "id": "b1b2c3d4-0000-4000-8000-000000000002",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "name": "one-b",
                            "position": 1,
                            "color": null,
                            "created_at": "2026-01-01T00:00:00Z"
                        },
                        {
                            "id": "b1b2c3d4-0000-4000-8000-000000000001",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "name": "one-a",
                            "position": 1,
                            "color": null,
                            "created_at": "2026-01-01T00:00:00Z"
                        }
                    ],
                    "tasks": [
                        {
                            "id": "c1b2c3d4-0000-4000-8000-00000000000a",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "title": "ten",
                            "description": null,
                            "priority": "medium",
                            "section_id": "b1b2c3d4-0000-4000-8000-000000000001",
                            "position": 10,
                            "lifecycle": { "state": "open" },
                            "tags": [],
                            "created_at": "2026-01-01T00:00:00Z"
                        },
                        {
                            "id": "c1b2c3d4-0000-4000-8000-00000000000c",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "title": "tie-c",
                            "description": null,
                            "priority": "medium",
                            "section_id": "b1b2c3d4-0000-4000-8000-000000000001",
                            "position": 2,
                            "lifecycle": { "state": "open" },
                            "tags": [],
                            "created_at": "2026-01-01T00:00:00Z"
                        },
                        {
                            "id": "c1b2c3d4-0000-4000-8000-00000000000b",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "title": "tie-b",
                            "description": null,
                            "priority": "medium",
                            "section_id": "b1b2c3d4-0000-4000-8000-000000000001",
                            "position": 2,
                            "lifecycle": { "state": "open" },
                            "tags": [],
                            "created_at": "2026-01-01T00:00:00Z"
                        },
                        {
                            "id": "c1b2c3d4-0000-4000-8000-00000000000d",
                            "project_id": "a1b2c3d4-0000-4000-8000-000000000001",
                            "title": "null-zero",
                            "description": null,
                            "priority": "medium",
                            "section_id": null,
                            "position": 0,
                            "lifecycle": { "state": "open" },
                            "tags": [],
                            "created_at": "2026-01-01T00:00:00Z"
                        }
                    ],
                    "notes": []
                }
            ]
        }"#;

        let backup = parse(v3).expect("version 3 should upgrade");
        assert_eq!(backup.version, 4);

        let mut sections = backup.projects[0].sections.clone();
        sections.sort_by_key(|entry| entry.position.clone());
        let section_names: Vec<&str> = sections.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(section_names, ["one-a", "one-b", "five"]);
        assert_eq!(sections[0].id, uuid(SECTION_ONE));
        assert_eq!(sections[1].id, uuid(SECTION_TWO));
        assert!(
            sections
                .windows(2)
                .all(|pair| pair[0].position < pair[1].position)
        );
        for entry in &sections {
            assert!(Position::parse(entry.position.as_str()).is_ok());
        }

        let mut section_tasks: Vec<&Task> = backup.projects[0]
            .tasks
            .iter()
            .filter(|task| task.section_id == Some(uuid(SECTION_ONE)))
            .collect();
        section_tasks.sort_by_key(|task| task.position.clone());
        let task_titles: Vec<&str> = section_tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect();
        assert_eq!(task_titles, ["tie-b", "tie-c", "ten"]);
        assert!(
            section_tasks
                .windows(2)
                .all(|pair| pair[0].position < pair[1].position)
        );
        for task in &section_tasks {
            assert!(Position::parse(task.position.as_str()).is_ok());
        }

        let unsectioned: Vec<&Task> = backup.projects[0]
            .tasks
            .iter()
            .filter(|task| task.section_id.is_none())
            .collect();
        assert_eq!(unsectioned.len(), 1);
        assert_eq!(unsectioned[0].title, "null-zero");
        assert!(Position::parse(unsectioned[0].position.as_str()).is_ok());
    }

    #[test]
    fn v4_backup_round_trips_string_positions() {
        let mut original = backup(vec![full_backup_project(PROJECT_ID, "alpha")]);
        original.projects[0].sections[0].position = position("a2");
        original.projects[0].tasks[0].position = position("a5");

        let serialized = serde_json::to_string(&original).expect("backup should serialize");
        let parsed = parse(&serialized).expect("version 4 should parse");

        assert_eq!(parsed.version, BACKUP_VERSION);
        assert_eq!(parsed.projects[0].sections[0].position, position("a2"));
        assert_eq!(parsed.projects[0].tasks[0].position, position("a5"));
    }
}
