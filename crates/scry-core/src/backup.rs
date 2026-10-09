use crate::error::StorageError;
use crate::models::{Note, Project, ProjectId, Status, Task};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const BACKUP_VERSION: u32 = 1;

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
    // the related collections, so the format reads naturally.
    #[serde(flatten)]
    pub project: Project,
    pub statuses: Vec<Status>,
    pub tasks: Vec<Task>,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Fail if any incoming project name already exists.
    Fail,
    /// Replace existing projects whose names collide with incoming ones.
    Replace,
    /// Skip incoming projects whose names already exist.
    Skip,
}

#[derive(Debug, Clone)]
pub struct ImportPlan {
    pub mode: ImportMode,
    /// Rows to insert, with ids/foreign keys already resolved.
    pub projects: Vec<BackupProject>,
    /// Ids of existing projects to delete before inserting (Replace mode only; empty otherwise).
    pub project_ids_to_delete: Vec<ProjectId>,
    /// New id for the active project when it was replaced (Replace mode only; None otherwise).
    pub active_project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ImportReport {
    pub projects: usize,
    pub statuses: usize,
    pub tasks: usize,
    pub notes: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IdMaxima {
    pub project: i64,
    pub status: i64,
    pub task: i64,
    pub note: i64,
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
        let statuses = store.get_all_statuses_by_project_id(project.id).await?;
        let tasks = store.get_all_tasks_by_project_id(project.id).await?;
        let notes = store.get_all_notes_by_project_id(project.id).await?;
        backup_projects.push(BackupProject {
            project,
            statuses,
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

    let mut maxima = IdMaxima {
        project: 0,
        status: 0,
        task: 0,
        note: 0,
    };
    for project in &existing {
        maxima.project = maxima.project.max(project.id);
        for status in store.get_all_statuses_by_project_id(project.id).await? {
            maxima.status = maxima.status.max(status.id);
        }
        for task in store.get_all_tasks_by_project_id(project.id).await? {
            maxima.task = maxima.task.max(task.id);
        }
        for note in store.get_all_notes_by_project_id(project.id).await? {
            maxima.note = maxima.note.max(note.id);
        }
    }

    match mode {
        ImportMode::Fail => plan_fail(backup, &existing_names, maxima),
        ImportMode::Skip => plan_skip(backup, &existing_names, maxima),
        ImportMode::Replace => {
            let active_name = store.get_active_project().await?.name;
            plan_replace(backup, &existing_ids_by_name, &active_name, maxima)
        }
    }
}

fn plan_fail(
    backup: &Backup,
    existing_names: &HashSet<String>,
    maxima: IdMaxima,
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
        projects: remap_projects(&backup.projects, maxima)?,
        project_ids_to_delete: Vec::new(),
        active_project_id: None,
    })
}

fn plan_skip(
    backup: &Backup,
    existing_names: &HashSet<String>,
    maxima: IdMaxima,
) -> Result<ImportPlan, StorageError> {
    let projects: Vec<BackupProject> = backup
        .projects
        .iter()
        .filter(|entry| !existing_names.contains(&entry.project.name))
        .cloned()
        .collect();

    Ok(ImportPlan {
        mode: ImportMode::Skip,
        projects: remap_projects(&projects, maxima)?,
        project_ids_to_delete: Vec::new(),
        active_project_id: None,
    })
}

fn plan_replace(
    backup: &Backup,
    existing_ids_by_name: &HashMap<String, ProjectId>,
    active_name: &str,
    maxima: IdMaxima,
) -> Result<ImportPlan, StorageError> {
    let projects = remap_projects(&backup.projects, maxima)?;

    let project_ids_to_delete = backup
        .projects
        .iter()
        .filter_map(|entry| existing_ids_by_name.get(&entry.project.name).copied())
        .collect();

    let active_project_id = projects
        .iter()
        .find(|entry| entry.project.name == active_name)
        .map(|entry| entry.project.id);

    Ok(ImportPlan {
        mode: ImportMode::Replace,
        projects,
        project_ids_to_delete,
        active_project_id,
    })
}

fn remap_projects(
    projects: &[BackupProject],
    maxima: IdMaxima,
) -> Result<Vec<BackupProject>, StorageError> {
    let mut next_status = maxima.status;
    let mut next_task = maxima.task;
    let mut next_note = maxima.note;

    let mut remapped = Vec::with_capacity(projects.len());

    for (index, backup_project) in projects.iter().enumerate() {
        let new_project_id = maxima.project + 1 + index as i64;

        let mut status_ids = HashMap::new();
        let mut statuses = Vec::with_capacity(backup_project.statuses.len());
        for status in &backup_project.statuses {
            next_status += 1;
            status_ids.insert(status.id, next_status);
            statuses.push(Status {
                id: next_status,
                project_id: new_project_id,
                name: status.name.clone(),
                position: status.position,
                color: status.color,
                style: status.style,
            });
        }

        let mut task_ids = HashMap::new();
        let mut tasks = Vec::with_capacity(backup_project.tasks.len());
        for task in &backup_project.tasks {
            let new_status_id = status_ids.get(&task.status_id).copied().ok_or_else(|| {
                StorageError::Invalid(format!(
                    "task {} references unknown status {}",
                    task.id, task.status_id
                ))
            })?;
            next_task += 1;
            task_ids.insert(task.id, next_task);
            tasks.push(Task {
                id: next_task,
                project_id: new_project_id,
                title: task.title.clone(),
                description: task.description.clone(),
                priority: task.priority,
                status_id: new_status_id,
                position: task.position,
                tags: task.tags.clone(),
                created_at: task.created_at,
            });
        }

        let mut notes = Vec::with_capacity(backup_project.notes.len());
        for note in &backup_project.notes {
            let new_task_id = task_ids.get(&note.task_id).copied().ok_or_else(|| {
                StorageError::Invalid(format!(
                    "note {} references unknown task {}",
                    note.id, note.task_id
                ))
            })?;
            next_note += 1;
            notes.push(Note {
                id: next_note,
                task_id: new_task_id,
                contents: note.contents.clone(),
                created_at: note.created_at,
            });
        }

        let entry_status_id = backup_project
            .project
            .entry_status_id
            .map(|old| {
                status_ids.get(&old).copied().ok_or_else(|| {
                    StorageError::Invalid(format!(
                        "project {} references unknown entry status {old}",
                        backup_project.project.id
                    ))
                })
            })
            .transpose()?;

        remapped.push(BackupProject {
            project: Project {
                id: new_project_id,
                name: backup_project.project.name.clone(),
                entry_status_id,
                task_sorting_mode: backup_project.project.task_sorting_mode,
                show_priority: backup_project.project.show_priority,
                created_at: backup_project.project.created_at,
            },
            statuses,
            tasks,
            notes,
        });
    }

    Ok(remapped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Priority, StatusStyle, Tags, TaskSortingMode};
    use chrono::Utc;

    fn project(id: ProjectId, name: &str, entry_status_id: Option<i64>) -> Project {
        Project {
            id,
            name: name.to_string(),
            entry_status_id,
            task_sorting_mode: TaskSortingMode::Manual,
            show_priority: false,
            created_at: Utc::now(),
        }
    }

    fn status(id: i64, project_id: ProjectId) -> Status {
        Status {
            id,
            project_id,
            name: format!("status-{id}"),
            position: 0,
            color: None,
            style: StatusStyle::None,
        }
    }

    fn task(id: i64, project_id: ProjectId, status_id: i64) -> Task {
        Task {
            id,
            project_id,
            title: format!("task-{id}"),
            description: None,
            priority: Priority::Medium,
            status_id,
            position: 0,
            tags: Tags::default(),
            created_at: Utc::now(),
        }
    }

    fn note(id: i64, task_id: i64) -> Note {
        Note {
            id,
            task_id,
            contents: format!("note-{id}"),
            created_at: Utc::now(),
        }
    }

    fn backup_project(id: ProjectId, name: &str, entry_status_id: Option<i64>) -> BackupProject {
        BackupProject {
            project: project(id, name, entry_status_id),
            statuses: vec![],
            tasks: vec![],
            notes: vec![],
        }
    }

    fn backup(projects: Vec<BackupProject>) -> Backup {
        Backup {
            version: BACKUP_VERSION,
            app_version: String::new(),
            projects,
        }
    }

    fn no_maxima() -> IdMaxima {
        IdMaxima {
            project: 0,
            status: 0,
            task: 0,
            note: 0,
        }
    }

    #[test]
    fn remap_projects_rewrites_ids_and_foreign_keys() {
        let projects = vec![BackupProject {
            project: project(5, "imported", Some(70)),
            statuses: vec![status(70, 5)],
            tasks: vec![task(30, 5, 70)],
            notes: vec![note(90, 30)],
        }];
        let maxima = IdMaxima {
            project: 100,
            status: 200,
            task: 300,
            note: 400,
        };

        let remapped = remap_projects(&projects, maxima).expect("remap should succeed");

        let imported = &remapped[0];
        assert_eq!(imported.project.id, 101);
        assert_eq!(imported.project.entry_status_id, Some(201));
        assert_eq!(imported.statuses[0].id, 201);
        assert_eq!(imported.statuses[0].project_id, 101);
        assert_eq!(imported.tasks[0].id, 301);
        assert_eq!(imported.tasks[0].project_id, 101);
        assert_eq!(imported.tasks[0].status_id, 201);
        assert_eq!(imported.notes[0].id, 401);
        assert_eq!(imported.notes[0].task_id, 301);
    }

    #[test]
    fn remap_projects_allocates_fresh_unique_ids_across_projects() {
        let projects = vec![
            BackupProject {
                project: project(1, "first", Some(10)),
                statuses: vec![status(10, 1)],
                tasks: vec![task(20, 1, 10)],
                notes: vec![note(30, 20)],
            },
            BackupProject {
                project: project(2, "second", Some(11)),
                statuses: vec![status(11, 2)],
                tasks: vec![task(21, 2, 11)],
                notes: vec![note(31, 21)],
            },
        ];
        let maxima = IdMaxima {
            project: 5,
            status: 6,
            task: 7,
            note: 8,
        };

        let remapped = remap_projects(&projects, maxima).expect("remap should succeed");

        let project_ids: Vec<_> = remapped.iter().map(|p| p.project.id).collect();
        let status_ids: Vec<_> = remapped
            .iter()
            .flat_map(|p| p.statuses.iter().map(|s| s.id))
            .collect();
        let task_ids: Vec<_> = remapped
            .iter()
            .flat_map(|p| p.tasks.iter().map(|t| t.id))
            .collect();
        let note_ids: Vec<_> = remapped
            .iter()
            .flat_map(|p| p.notes.iter().map(|n| n.id))
            .collect();

        assert!(project_ids.iter().all(|id| *id > maxima.project));
        assert!(status_ids.iter().all(|id| *id > maxima.status));
        assert!(task_ids.iter().all(|id| *id > maxima.task));
        assert!(note_ids.iter().all(|id| *id > maxima.note));

        for ids in [&project_ids, &status_ids, &task_ids, &note_ids] {
            let unique: HashSet<_> = ids.iter().collect();
            assert_eq!(unique.len(), ids.len(), "ids must not collide: {ids:?}");
        }
    }

    #[test]
    fn remap_projects_rejects_task_with_unknown_status() {
        let projects = vec![BackupProject {
            project: project(1, "imported", None),
            statuses: vec![status(10, 1)],
            tasks: vec![task(20, 1, 999)],
            notes: vec![],
        }];

        assert!(matches!(
            remap_projects(&projects, no_maxima()),
            Err(StorageError::Invalid(_))
        ));
    }

    #[test]
    fn remap_projects_rejects_note_with_unknown_task() {
        let projects = vec![BackupProject {
            project: project(1, "imported", None),
            statuses: vec![status(10, 1)],
            tasks: vec![task(20, 1, 10)],
            notes: vec![note(30, 999)],
        }];

        assert!(matches!(
            remap_projects(&projects, no_maxima()),
            Err(StorageError::Invalid(_))
        ));
    }

    #[test]
    fn remap_projects_rejects_project_with_unknown_entry_status() {
        let projects = vec![BackupProject {
            project: project(1, "imported", Some(999)),
            statuses: vec![status(10, 1)],
            tasks: vec![],
            notes: vec![],
        }];

        assert!(matches!(
            remap_projects(&projects, no_maxima()),
            Err(StorageError::Invalid(_))
        ));
    }

    #[test]
    fn plan_fail_rejects_colliding_names() {
        let backup = backup(vec![
            backup_project(1, "keep", None),
            backup_project(2, "collide", None),
        ]);
        let existing = HashSet::from(["collide".to_string()]);

        assert!(matches!(
            plan_fail(&backup, &existing, no_maxima()),
            Err(StorageError::Conflict(_))
        ));
    }

    #[test]
    fn plan_fail_remaps_all_projects_when_no_collision() {
        let backup = backup(vec![
            backup_project(1, "alpha", None),
            backup_project(2, "beta", None),
        ]);

        let plan = plan_fail(&backup, &HashSet::new(), no_maxima())
            .expect("fail should succeed when clear");

        assert_eq!(plan.mode, ImportMode::Fail);
        assert_eq!(plan.projects.len(), 2);
        assert!(plan.project_ids_to_delete.is_empty());
        assert_eq!(plan.active_project_id, None);
    }

    #[test]
    fn plan_skip_drops_colliding_projects() {
        let backup = backup(vec![
            backup_project(1, "keep", None),
            backup_project(2, "skip", None),
        ]);
        let existing = HashSet::from(["skip".to_string()]);

        let plan = plan_skip(&backup, &existing, no_maxima()).expect("skip should succeed");

        assert_eq!(plan.mode, ImportMode::Skip);
        assert_eq!(plan.projects.len(), 1);
        assert_eq!(plan.projects[0].project.name, "keep");
        assert!(plan.project_ids_to_delete.is_empty());
        assert_eq!(plan.active_project_id, None);
    }

    #[test]
    fn plan_replace_lists_collisions_and_rehomes_active() {
        let backup = backup(vec![
            backup_project(1, "alpha", None),
            backup_project(2, "beta", None),
        ]);
        let existing = HashMap::from([("alpha".to_string(), 10), ("other".to_string(), 11)]);

        let plan =
            plan_replace(&backup, &existing, "alpha", no_maxima()).expect("replace should succeed");

        assert_eq!(plan.mode, ImportMode::Replace);
        assert_eq!(plan.projects.len(), 2);
        assert_eq!(plan.project_ids_to_delete, vec![10]);

        let new_alpha = plan
            .projects
            .iter()
            .find(|entry| entry.project.name == "alpha")
            .expect("alpha should remain")
            .project
            .id;
        assert!(new_alpha > 0);
        assert_eq!(plan.active_project_id, Some(new_alpha));
    }

    #[test]
    fn plan_replace_has_no_active_when_active_name_absent() {
        let backup = backup(vec![backup_project(1, "alpha", None)]);
        let existing = HashMap::from([("alpha".to_string(), 10)]);

        let plan =
            plan_replace(&backup, &existing, "gamma", no_maxima()).expect("replace should succeed");

        assert_eq!(plan.active_project_id, None);
    }
}
