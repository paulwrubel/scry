use crate::error::StorageError;
use crate::ids::{AutomationId, CommentId, ProjectId, SectionId, TaskId};
use crate::models::{
    Automation, AutomationAction, AutomationTrigger, Color, Comment, EntityType,
    MAX_AUTOMATION_ACTIONS, Project, ProjectSnapshot, Section, Tags, Task,
};
use crate::store::Store;
use crate::sync::operations::{
    ColorChange as OperationColorChange, OPERATION_VERSION, Operation, OperationKind,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

pub const SNAPSHOT_VERSION: u32 = 3;
pub const MIN_SUPPORTED_SNAPSHOT_VERSION: u32 = 2;

/// The bare server-wide snapshot: every project document plus the cursor a
/// client should pull from next.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
                "unsupported snapshot version {found}; this build supports versions {MIN_SUPPORTED_SNAPSHOT_VERSION} through {supported}"
            ),
        }
    }
}

impl std::error::Error for SnapshotFileError {}

/// Parse a snapshot file.
///
/// The current format version and the immediately preceding one are accepted;
/// there is no upgrade path for older versions, so a document outside that
/// range is rejected rather than guessed at.
pub fn parse_file(input: &str) -> Result<SnapshotFile, SnapshotFileError> {
    let file: SnapshotFile = serde_json::from_str(input)
        .map_err(|error| SnapshotFileError::InvalidJson(error.to_string()))?;
    if !(MIN_SUPPORTED_SNAPSHOT_VERSION..=SNAPSHOT_VERSION).contains(&file.version) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportMode {
    /// Merge into the server: upsert same-id entities, remint tombstoned ids,
    /// and auto-rename project-name collisions.
    Merge,
    /// Stricter legacy mode: a different live project with the same name is an error.
    Fail,
    /// Replace existing projects whose names collide.
    Replace,
    /// Skip incoming projects whose names collide with live projects.
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportIdRemap {
    pub entity_type: EntityType,
    pub from: Uuid,
    pub to: Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportReport {
    pub projects: usize,
    pub sections: usize,
    pub tasks: usize,
    pub comments: usize,
    pub automations: usize,
    /// Fresh ids assigned because the incoming id was tombstoned or its parent
    /// was reminted. Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remapped: Vec<ImportIdRemap>,
}

/// All live data plus the tombstones relevant to the incoming file.
#[derive(Debug, Clone, Default)]
pub struct ImportState {
    pub projects: Vec<Project>,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub comments: Vec<Comment>,
    pub automations: Vec<Automation>,
    pub tombstones: HashSet<(EntityType, Uuid)>,
}

#[derive(Debug, Clone)]
pub struct ImportPlan {
    pub operations: Vec<Operation>,
    pub report: ImportReport,
}

/// Turn a snapshot file into operations against the current server state.
///
/// Planning is pure: the caller supplies the live rows and tombstones, so the
/// whole import can be reviewed and applied as one atomic batch.
pub fn plan_import(
    file: &SnapshotFile,
    state: &ImportState,
    mode: ImportMode,
) -> Result<ImportPlan, StorageError> {
    let deleted_project_ids = replace_deletions(file, state, mode);

    let deleted_projects: HashSet<ProjectId> = deleted_project_ids.iter().copied().collect();
    let task_project_ids: HashMap<TaskId, ProjectId> = state
        .tasks
        .iter()
        .map(|task| (task.id, task.project_id))
        .collect();

    let mut deleted_ids: HashSet<(EntityType, Uuid)> = HashSet::new();
    for project_id in &deleted_project_ids {
        deleted_ids.insert((EntityType::Project, *project_id));
    }
    for section in &state.sections {
        if deleted_projects.contains(&section.project_id) {
            deleted_ids.insert((EntityType::Section, section.id));
        }
    }
    for task in &state.tasks {
        if deleted_projects.contains(&task.project_id) {
            deleted_ids.insert((EntityType::Task, task.id));
        }
    }
    for comment in &state.comments {
        if task_project_ids
            .get(&comment.task_id)
            .is_some_and(|project_id| deleted_projects.contains(project_id))
        {
            deleted_ids.insert((EntityType::Comment, comment.id));
        }
    }
    for automation in &state.automations {
        if deleted_projects.contains(&automation.project_id) {
            deleted_ids.insert((EntityType::Automation, automation.id));
        }
    }

    let is_dead = |entity_type: EntityType, id: Uuid| {
        state.tombstones.contains(&(entity_type, id)) || deleted_ids.contains(&(entity_type, id))
    };

    let live_projects: HashMap<ProjectId, &Project> = state
        .projects
        .iter()
        .filter(|project| !deleted_ids.contains(&(EntityType::Project, project.id)))
        .map(|project| (project.id, project))
        .collect();
    let live_sections: HashMap<SectionId, &Section> = state
        .sections
        .iter()
        .filter(|section| !deleted_ids.contains(&(EntityType::Section, section.id)))
        .map(|section| (section.id, section))
        .collect();
    let live_tasks: HashMap<TaskId, &Task> = state
        .tasks
        .iter()
        .filter(|task| !deleted_ids.contains(&(EntityType::Task, task.id)))
        .map(|task| (task.id, task))
        .collect();
    let live_comments: HashMap<CommentId, &Comment> = state
        .comments
        .iter()
        .filter(|comment| !deleted_ids.contains(&(EntityType::Comment, comment.id)))
        .map(|comment| (comment.id, comment))
        .collect();
    let live_automations: HashMap<AutomationId, &Automation> = state
        .automations
        .iter()
        .filter(|automation| !deleted_ids.contains(&(EntityType::Automation, automation.id)))
        .map(|automation| (automation.id, automation))
        .collect();

    // imported entities must stay inside the project their ops target, so track
    // the effective project of every section and task the batch will create
    let mut section_projects: HashMap<SectionId, ProjectId> = live_sections
        .iter()
        .map(|(id, section)| (*id, section.project_id))
        .collect();
    let mut task_projects: HashMap<TaskId, ProjectId> = live_tasks
        .iter()
        .map(|(id, task)| (*id, task.project_id))
        .collect();

    let mut project_names: HashMap<ProjectId, String> = live_projects
        .iter()
        .map(|(id, project)| (*id, project.name.clone()))
        .collect();
    let mut section_names: HashMap<ProjectId, HashMap<SectionId, String>> = HashMap::new();
    for section in live_sections.values() {
        section_names
            .entry(section.project_id)
            .or_default()
            .insert(section.id, section.name.clone());
    }

    let mut plan = ImportPlan {
        operations: Vec::new(),
        report: ImportReport::default(),
    };

    // deletions come first so the rest of the batch sees the freed names and ids
    for project_id in &deleted_project_ids {
        plan.operations
            .push(operation(OperationKind::DeleteProject { id: *project_id }));
    }

    let mut section_remap: HashMap<SectionId, SectionId> = HashMap::new();
    let mut task_remap: HashMap<TaskId, TaskId> = HashMap::new();

    for document in &file.projects {
        let incoming = &document.project;
        let project_dead = is_dead(EntityType::Project, incoming.id);
        let live_project = live_projects
            .get(&incoming.id)
            .copied()
            .filter(|_| !project_dead);

        if live_project.is_none() {
            match mode {
                ImportMode::Skip => {
                    if project_name_taken(&project_names, &incoming.name, None) {
                        continue;
                    }
                }
                ImportMode::Fail => {
                    if project_name_taken(&project_names, &incoming.name, None) {
                        let name = &incoming.name;
                        return Err(StorageError::Conflict(format!(
                            "project(s) already exist: {name}"
                        )));
                    }
                }
                ImportMode::Merge | ImportMode::Replace => {}
            }
        }

        let chosen_name = match live_project {
            Some(live_project) => unique_name(&incoming.name, |candidate| {
                project_name_taken(&project_names, candidate, Some(live_project.id))
            }),
            None if matches!(mode, ImportMode::Merge | ImportMode::Replace) => {
                unique_name(&incoming.name, |candidate| {
                    project_name_taken(&project_names, candidate, None)
                })
            }
            None => incoming.name.clone(),
        };

        // apply-time creates drop empty descriptions, so compare the same normalized form
        let description = incoming
            .description
            .clone()
            .filter(|description| !description.is_empty());

        let (target_project_id, project_reminted) = if let Some(live_project) = live_project {
            let name = changed(&live_project.name, &chosen_name);
            let description = changed(&live_project.description, &description);

            if name.is_some() || description.is_some() {
                plan.operations
                    .push(operation(OperationKind::UpdateProject {
                        id: live_project.id,
                        name,
                        description,
                    }));
            }

            (live_project.id, false)
        } else {
            let project_id = if project_dead {
                let fresh = Uuid::new_v4();
                plan.report.remapped.push(ImportIdRemap {
                    entity_type: EntityType::Project,
                    from: incoming.id,
                    to: fresh,
                });
                fresh
            } else {
                incoming.id
            };

            plan.operations
                .push(operation(OperationKind::CreateProject {
                    id: project_id,
                    name: chosen_name.clone(),
                    description,
                    created_at: incoming.created_at,
                }));

            (project_id, project_dead)
        };

        project_names.insert(target_project_id, chosen_name);
        plan.report.projects += 1;

        let names = section_names.entry(target_project_id).or_default();

        for incoming_section in &document.sections {
            let original_id = incoming_section.id;
            let section_dead = is_dead(EntityType::Section, original_id);

            if project_reminted || section_dead {
                // a tombstoned id can merge into a live section of the same name,
                // which is how a diverged backup rejoins its project
                let reuse = if project_reminted {
                    None
                } else {
                    live_section_by_name(
                        &live_sections,
                        names,
                        target_project_id,
                        &incoming_section.name,
                    )
                };

                if let Some(live_section) = reuse {
                    section_remap.insert(original_id, live_section.id);
                    if let Some(color) =
                        section_color_change(live_section.color, incoming_section.color)
                    {
                        plan.operations
                            .push(operation(OperationKind::UpdateSection {
                                id: live_section.id,
                                name: None,
                                color: Some(color),
                            }));
                    }
                    plan.report.sections += 1;
                    continue;
                }

                let section_id = Uuid::new_v4();
                plan.report.remapped.push(ImportIdRemap {
                    entity_type: EntityType::Section,
                    from: original_id,
                    to: section_id,
                });
                section_remap.insert(original_id, section_id);
                section_projects.insert(section_id, target_project_id);

                let name = unique_name(&incoming_section.name, |candidate| {
                    names.values().any(|existing| existing == candidate)
                });
                names.insert(section_id, name.clone());

                plan.operations
                    .push(operation(OperationKind::CreateSection {
                        id: section_id,
                        project_id: target_project_id,
                        name,
                        position: incoming_section.position.clone(),
                        color: incoming_section.color,
                        created_at: incoming_section.created_at,
                    }));
                plan.report.sections += 1;
                continue;
            }

            if let Some(live_section) = live_sections.get(&original_id).copied() {
                if live_section.project_id != target_project_id {
                    return Err(StorageError::Conflict(format!(
                        "section {original_id} belongs to a different project"
                    )));
                }

                let name = unique_name(&incoming_section.name, |candidate| {
                    section_name_taken(names, candidate, Some(live_section.id))
                });
                let color = section_color_change(live_section.color, incoming_section.color);
                let renamed = name != live_section.name;

                if renamed || color.is_some() {
                    plan.operations
                        .push(operation(OperationKind::UpdateSection {
                            id: live_section.id,
                            name: renamed.then(|| name.clone()),
                            color,
                        }));
                }

                names.insert(live_section.id, name);
                plan.report.sections += 1;
                continue;
            }

            let reused = live_section_by_name(
                &live_sections,
                names,
                target_project_id,
                &incoming_section.name,
            );

            if let Some(live_section) = reused {
                section_remap.insert(original_id, live_section.id);
                if let Some(color) =
                    section_color_change(live_section.color, incoming_section.color)
                {
                    plan.operations
                        .push(operation(OperationKind::UpdateSection {
                            id: live_section.id,
                            name: None,
                            color: Some(color),
                        }));
                }
                plan.report.sections += 1;
                continue;
            }

            let name = unique_name(&incoming_section.name, |candidate| {
                names.values().any(|existing| existing == candidate)
            });
            names.insert(original_id, name.clone());
            section_projects.insert(original_id, target_project_id);
            plan.operations
                .push(operation(OperationKind::CreateSection {
                    id: original_id,
                    project_id: target_project_id,
                    name,
                    position: incoming_section.position.clone(),
                    color: incoming_section.color,
                    created_at: incoming_section.created_at,
                }));
            plan.report.sections += 1;
        }

        for incoming_task in &document.tasks {
            let original_id = incoming_task.id;
            let task_dead = is_dead(EntityType::Task, original_id);

            let section_id = incoming_task.section_id.map(|section_id| {
                section_remap
                    .get(&section_id)
                    .copied()
                    .unwrap_or(section_id)
            });

            if section_id.is_some_and(|section_id| {
                section_projects.get(&section_id) != Some(&target_project_id)
            }) {
                return Err(StorageError::Conflict(format!(
                    "task {original_id} references a section outside project {target_project_id}"
                )));
            }

            // apply-time creates drop empty descriptions, so compare the same normalized form
            let description = incoming_task
                .description
                .clone()
                .filter(|description| !description.is_empty());

            if project_reminted || task_dead {
                let task_id = Uuid::new_v4();
                plan.report.remapped.push(ImportIdRemap {
                    entity_type: EntityType::Task,
                    from: original_id,
                    to: task_id,
                });
                task_remap.insert(original_id, task_id);
                task_projects.insert(task_id, target_project_id);

                plan.operations.push(operation(OperationKind::CreateTask {
                    id: task_id,
                    project_id: target_project_id,
                    title: incoming_task.title.clone(),
                    description,
                    priority: incoming_task.priority,
                    section_id,
                    position: incoming_task.position.clone(),
                    state: incoming_task.state,
                    tags: incoming_task.tags.clone(),
                    created_at: incoming_task.created_at,
                    external_identity: incoming_task.external_identity.clone(),
                    external_metadata: incoming_task.external_metadata.clone().map(Box::new),
                }));
                plan.report.tasks += 1;
                continue;
            }

            if let Some(live_task) = live_tasks.get(&original_id).copied() {
                if live_task.project_id != target_project_id {
                    return Err(StorageError::Conflict(format!(
                        "task {original_id} belongs to a different project"
                    )));
                }

                let title = changed(&live_task.title, &incoming_task.title);
                let description = changed(&live_task.description, &description);
                let priority = changed(&live_task.priority, &incoming_task.priority);
                let tags = tags_differ(&live_task.tags, &incoming_task.tags)
                    .then(|| incoming_task.tags.clone());
                let state = changed(&live_task.state, &incoming_task.state);

                if title.is_some()
                    || description.is_some()
                    || priority.is_some()
                    || tags.is_some()
                    || state.is_some()
                {
                    plan.operations.push(operation(OperationKind::UpdateTask {
                        id: live_task.id,
                        title,
                        description,
                        priority,
                        tags,
                        state,
                        external_identity: None,
                        external_metadata: None,
                    }));
                }

                if live_task.section_id != section_id
                    || live_task.position != incoming_task.position
                {
                    plan.operations.push(operation(OperationKind::MoveTask {
                        id: live_task.id,
                        section_id,
                        position: incoming_task.position.clone(),
                    }));
                }

                plan.report.tasks += 1;
                continue;
            }

            task_projects.insert(original_id, target_project_id);
            plan.operations.push(operation(OperationKind::CreateTask {
                id: original_id,
                project_id: target_project_id,
                title: incoming_task.title.clone(),
                description,
                priority: incoming_task.priority,
                section_id,
                position: incoming_task.position.clone(),
                state: incoming_task.state,
                tags: incoming_task.tags.clone(),
                created_at: incoming_task.created_at,
                external_identity: incoming_task.external_identity.clone(),
                external_metadata: incoming_task.external_metadata.clone().map(Box::new),
            }));
            plan.report.tasks += 1;
        }

        for incoming_comment in &document.comments {
            let original_id = incoming_comment.id;
            let original_task_id = incoming_comment.task_id;
            let task_remapped = task_remap.get(&original_task_id).copied();

            let task_id = task_remapped.unwrap_or(original_task_id);

            if task_projects.get(&task_id) != Some(&target_project_id) {
                return Err(StorageError::Conflict(format!(
                    "comment {original_id} references a task outside project {target_project_id}"
                )));
            }

            let comment_dead = is_dead(EntityType::Comment, original_id);
            let reminted = project_reminted || comment_dead || task_remapped.is_some();

            let live_comment = live_comments
                .get(&original_id)
                .copied()
                .filter(|_| !reminted);

            if let Some(live_comment) = live_comment {
                if live_comment.task_id != task_id {
                    return Err(StorageError::Conflict(format!(
                        "comment {original_id} belongs to a different task"
                    )));
                }

                if live_comment.contents != incoming_comment.contents {
                    plan.operations
                        .push(operation(OperationKind::UpdateComment {
                            id: live_comment.id,
                            contents: incoming_comment.contents.clone(),
                        }));
                }

                plan.report.comments += 1;
                continue;
            }

            let comment_id = if reminted {
                let fresh = Uuid::new_v4();
                plan.report.remapped.push(ImportIdRemap {
                    entity_type: EntityType::Comment,
                    from: original_id,
                    to: fresh,
                });
                fresh
            } else {
                original_id
            };

            plan.operations
                .push(operation(OperationKind::CreateComment {
                    id: comment_id,
                    task_id,
                    contents: incoming_comment.contents.clone(),
                    created_at: incoming_comment.created_at,
                }));
            plan.report.comments += 1;
        }

        for incoming_automation in &document.automations {
            let original_id = incoming_automation.id;

            let trigger = remap_automation_trigger(&incoming_automation.trigger, &section_remap);
            let actions: Vec<AutomationAction> = incoming_automation
                .actions
                .iter()
                .map(|action| remap_automation_action(action, &section_remap))
                .collect();

            if actions.is_empty() || actions.len() > MAX_AUTOMATION_ACTIONS {
                return Err(StorageError::Conflict(format!(
                    "automation {original_id} has an invalid number of actions"
                )));
            }

            let section_ids = trigger
                .section_id()
                .into_iter()
                .chain(actions.iter().filter_map(AutomationAction::section_id));
            for section_id in section_ids {
                if section_projects.get(&section_id) != Some(&target_project_id) {
                    return Err(StorageError::Conflict(format!(
                        "automation {original_id} references a section outside project {target_project_id}"
                    )));
                }
            }

            let dead = is_dead(EntityType::Automation, original_id);
            let reminted = project_reminted || dead;
            let live_automation = live_automations
                .get(&original_id)
                .copied()
                .filter(|_| !reminted);

            if let Some(live_automation) = live_automation {
                if live_automation.project_id != target_project_id {
                    return Err(StorageError::Conflict(format!(
                        "automation {original_id} belongs to a different project"
                    )));
                }
                if live_automation.trigger != trigger {
                    return Err(StorageError::Conflict(format!(
                        "automation {original_id} changes an immutable trigger"
                    )));
                }

                if live_automation.actions != actions {
                    plan.operations
                        .push(operation(OperationKind::UpdateAutomation {
                            id: live_automation.id,
                            actions,
                        }));
                }

                plan.report.automations += 1;
                continue;
            }

            if live_automations.values().any(|existing| {
                existing.project_id == target_project_id
                    && existing.id != original_id
                    && existing.trigger == trigger
            }) {
                return Err(StorageError::Conflict(format!(
                    "automation {original_id} duplicates a trigger in project {target_project_id}"
                )));
            }

            let automation_id = if reminted {
                let fresh = Uuid::new_v4();
                plan.report.remapped.push(ImportIdRemap {
                    entity_type: EntityType::Automation,
                    from: original_id,
                    to: fresh,
                });
                fresh
            } else {
                original_id
            };

            plan.operations
                .push(operation(OperationKind::CreateAutomation {
                    id: automation_id,
                    project_id: target_project_id,
                    trigger,
                    actions,
                    created_at: incoming_automation.created_at,
                }));
            plan.report.automations += 1;
        }
    }

    Ok(plan)
}

fn replace_deletions(file: &SnapshotFile, state: &ImportState, mode: ImportMode) -> Vec<ProjectId> {
    if mode != ImportMode::Replace {
        return Vec::new();
    }

    let live_ids: HashSet<ProjectId> = state.projects.iter().map(|project| project.id).collect();
    let mut deleted = Vec::new();
    // one live project can collide with several incoming documents
    let mut seen = HashSet::new();

    for document in &file.projects {
        let incoming = &document.project;
        // a live id is an upsert, so its same-named siblings must survive
        if live_ids.contains(&incoming.id) {
            continue;
        }

        for live in &state.projects {
            if live.name == incoming.name && live.id != incoming.id && seen.insert(live.id) {
                deleted.push(live.id);
            }
        }
    }

    deleted
}

fn operation(kind: OperationKind) -> Operation {
    Operation {
        operation_id: Uuid::new_v4(),
        operation_version: OPERATION_VERSION,
        timestamp: None,
        kind,
    }
}

fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base} ({suffix})");
        if !taken(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn project_name_taken(
    names: &HashMap<ProjectId, String>,
    name: &str,
    except: Option<ProjectId>,
) -> bool {
    names
        .iter()
        .any(|(id, existing)| Some(*id) != except && existing == name)
}

fn section_name_taken(
    names: &HashMap<SectionId, String>,
    name: &str,
    except: Option<SectionId>,
) -> bool {
    names
        .iter()
        .any(|(id, existing)| Some(*id) != except && existing == name)
}

/// The lowest-id live section in `target_project_id` whose current planned name
/// is `name`, if one exists.
fn live_section_by_name<'a>(
    live_sections: &HashMap<SectionId, &'a Section>,
    names: &HashMap<SectionId, String>,
    target_project_id: ProjectId,
    name: &str,
) -> Option<&'a Section> {
    live_sections
        .values()
        .copied()
        .filter(|section| section.project_id == target_project_id)
        .filter(|section| {
            names
                .get(&section.id)
                .is_some_and(|existing| existing.as_str() == name)
        })
        .min_by_key(|section| section.id)
}

fn changed<T: PartialEq + Clone>(current: &T, incoming: &T) -> Option<T> {
    (current != incoming).then(|| incoming.clone())
}

fn tags_differ(existing: &Tags, incoming: &Tags) -> bool {
    !existing.iter().eq(incoming.iter())
}

fn section_color_change(
    current: Option<Color>,
    incoming: Option<Color>,
) -> Option<OperationColorChange> {
    if current == incoming {
        return None;
    }

    Some(match incoming {
        Some(color) => OperationColorChange::Set(color),
        None => OperationColorChange::Clear,
    })
}

fn remap_automation_trigger(
    trigger: &AutomationTrigger,
    section_remap: &HashMap<SectionId, SectionId>,
) -> AutomationTrigger {
    match trigger {
        AutomationTrigger::TaskEntersSection { section_id } => {
            AutomationTrigger::TaskEntersSection {
                section_id: section_remap
                    .get(section_id)
                    .copied()
                    .unwrap_or(*section_id),
            }
        }
        other => other.clone(),
    }
}

fn remap_automation_action(
    action: &AutomationAction,
    section_remap: &HashMap<SectionId, SectionId>,
) -> AutomationAction {
    match action {
        AutomationAction::MoveToSection {
            section_id: Some(section_id),
        } => AutomationAction::MoveToSection {
            section_id: Some(
                section_remap
                    .get(section_id)
                    .copied()
                    .unwrap_or(*section_id),
            ),
        },
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Comment, ExternalIdentity, ExternalMetadata, Priority, Project, Section, Tags, Task,
        TaskState,
    };
    use crate::position::Position;
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use uuid::Uuid;

    const PROJECT_ID: &str = "a1b2c3d4-0000-4000-8000-000000000001";
    const SECTION_ID: &str = "b1b2c3d4-0000-4000-8000-000000000003";
    const TASK_ID: &str = "c1b2c3d4-0000-4000-8000-000000000004";
    const COMMENT_ID: &str = "d1b2c3d4-0000-4000-8000-000000000005";

    fn uuid(value: &str) -> Uuid {
        Uuid::parse_str(value).expect("test uuid should parse")
    }

    fn position(value: &str) -> Position {
        Position::parse(value).expect("test position should parse")
    }

    fn at() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).expect("valid timestamp")
    }

    fn project(id: &str, name: &str) -> Project {
        Project {
            id: uuid(id),
            name: name.to_string(),
            description: None,
            created_at: at(),
        }
    }

    fn section(id: &str, project_id: Uuid) -> Section {
        Section {
            id: uuid(id),
            project_id,
            name: "section".to_string(),
            position: position("a0"),
            color: None,
            created_at: at(),
        }
    }

    fn task(id: &str, project_id: Uuid, section_id: Option<Uuid>) -> Task {
        Task {
            id: uuid(id),
            project_id,
            title: "task".to_string(),
            description: None,
            priority: Priority::Normal,
            section_id,
            position: position("a0"),
            state: TaskState::Open,
            tags: Tags::default(),
            created_at: at(),
            external_identity: None,
            external_metadata: None,
        }
    }

    fn comment(id: &str, task_id: Uuid) -> Comment {
        Comment {
            id: uuid(id),
            task_id,
            contents: "note".to_string(),
            created_at: at(),
        }
    }

    fn full_project_doc(id: &str, name: &str) -> ProjectSnapshot {
        let project = project(id, name);
        let section = section(SECTION_ID, project.id);
        let task = task(TASK_ID, project.id, Some(section.id));
        let comment = comment(COMMENT_ID, task.id);

        ProjectSnapshot {
            project,
            sections: vec![section],
            tasks: vec![task],
            comments: vec![comment],
            automations: Vec::new(),
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

    #[test]
    fn parse_accepts_previous_version() {
        let file = snapshot_file(vec![full_project_doc(PROJECT_ID, "alpha")]);
        let mut value = serde_json::to_value(&file).expect("snapshot file should serialize");
        value["version"] = json!(2);
        for project in value["projects"]
            .as_array_mut()
            .expect("projects should be an array")
        {
            let project = project
                .as_object_mut()
                .expect("project should be an object");
            project.remove("automations");
            // v2 named the comment list "notes"; the alias keeps it readable
            let comments = project.remove("comments").expect("comments should exist");
            project.insert("notes".to_string(), comments);
        }

        let parsed = parse_file(&value.to_string()).expect("previous version should parse");

        assert_eq!(parsed.version, 2);
        assert!(parsed.projects[0].automations.is_empty());
        assert_eq!(parsed.projects[0].comments.len(), 1);
    }

    #[test]
    fn snapshot_file_round_trips_automations() {
        let mut document = full_project_doc(PROJECT_ID, "alpha");
        let section_id = document.sections[0].id;
        document.automations.push(Automation {
            id: uuid("e1b2c3d4-0000-4000-8000-000000000006"),
            project_id: document.project.id,
            trigger: AutomationTrigger::TaskEntersSection { section_id },
            actions: vec![AutomationAction::MoveToSection {
                section_id: Some(section_id),
            }],
            created_at: at(),
        });
        let original = snapshot_file(vec![document]);

        let serialized = serde_json::to_string(&original).expect("snapshot file should serialize");
        let parsed = parse_file(&serialized).expect("current version should parse");

        assert_eq!(
            parsed.projects[0].automations,
            original.projects[0].automations
        );
    }

    #[test]
    fn linked_task_round_trips_external_fields() {
        let identity = ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 42,
        };
        let metadata = ExternalMetadata::Github {
            title: Some("Widget".to_string()),
            description: Some("A widget".to_string()),
            kind: Some("issue".to_string()),
            state: Some("closed".to_string()),
            reason: Some("completed".to_string()),
            draft: None,
            updated_at: Some(at()),
            error: None,
        };

        let mut document = full_project_doc(PROJECT_ID, "alpha");
        document.tasks[0].external_identity = Some(identity.clone());
        document.tasks[0].external_metadata = Some(metadata.clone());

        let original = snapshot_file(vec![document]);
        let serialized = serde_json::to_string(&original).expect("snapshot file should serialize");
        let parsed = parse_file(&serialized).expect("current version should parse");

        let task = &parsed.projects[0].tasks[0];
        assert_eq!(task.external_identity.as_ref(), Some(&identity));
        assert_eq!(task.external_metadata.as_ref(), Some(&metadata));
    }

    #[test]
    fn old_task_json_defaults_external_fields() {
        let task: Task = serde_json::from_value(json!({
            "id": TASK_ID,
            "project_id": PROJECT_ID,
            "title": "old",
            "description": null,
            "priority": "normal",
            "section_id": null,
            "position": "a0",
            "state": "open",
            "tags": [],
            "created_at": "1970-01-01T00:00:00Z"
        }))
        .expect("a task without external fields should deserialize");

        assert!(task.external_identity.is_none());
        assert!(task.external_metadata.is_none());
    }

    #[test]
    fn legacy_priority_names_deserialize() {
        let parse = |priority: &str| -> Task {
            serde_json::from_value(json!({
                "id": TASK_ID,
                "project_id": PROJECT_ID,
                "title": "old",
                "description": null,
                "priority": priority,
                "section_id": null,
                "position": "a0",
                "state": "open",
                "tags": [],
                "created_at": "1970-01-01T00:00:00Z"
            }))
            .expect("a task with a legacy priority name should deserialize")
        };

        assert_eq!(parse("critical").priority, Priority::Critical);
        assert_eq!(parse("high").priority, Priority::Elevated);
        assert_eq!(parse("medium").priority, Priority::Normal);
        assert_eq!(parse("low").priority, Priority::Normal);
        assert_eq!(parse("minimal").priority, Priority::Normal);
    }

    #[test]
    fn old_project_json_with_task_sorting_mode_still_deserializes() {
        let document: ProjectSnapshot = serde_json::from_value(json!({
            "id": PROJECT_ID,
            "name": "alpha",
            "description": null,
            "task_sorting_mode": "priority",
            "show_priority": false,
            "created_at": "1970-01-01T00:00:00Z",
            "sections": [],
            "tasks": [],
            "comments": [],
            "automations": []
        }))
        .expect("a snapshot carrying the removed task_sorting_mode field should deserialize");

        assert_eq!(document.project.id, uuid(PROJECT_ID));
        assert_eq!(document.project.name, "alpha");
        assert_eq!(document.project.created_at, at());
    }

    #[test]
    fn serialized_project_snapshot_omits_task_sorting_mode() {
        let serialized = serde_json::to_value(full_project_doc(PROJECT_ID, "alpha"))
            .expect("snapshot document should serialize");

        assert!(
            serialized
                .as_object()
                .expect("project document should be an object")
                .get("task_sorting_mode")
                .is_none()
        );
    }

    #[test]
    fn serialized_project_snapshot_omits_show_priority() {
        let serialized = serde_json::to_value(full_project_doc(PROJECT_ID, "alpha"))
            .expect("snapshot document should serialize");

        assert!(
            serialized
                .as_object()
                .expect("project document should be an object")
                .get("show_priority")
                .is_none()
        );
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::models::{ExternalIdentity, ExternalMetadata, Priority, ProjectSnapshot, TaskState};
    use crate::position::Position;
    use chrono::{DateTime, Utc};

    fn uuid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn at() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).expect("valid timestamp")
    }

    fn project(id: Uuid, name: &str) -> Project {
        Project {
            id,
            name: name.to_string(),
            description: None,
            created_at: at(),
        }
    }

    fn section(id: Uuid, project_id: Uuid, name: &str) -> Section {
        Section {
            id,
            project_id,
            name: name.to_string(),
            position: Position::parse("a0").unwrap(),
            color: None,
            created_at: at(),
        }
    }

    fn task(id: Uuid, project_id: Uuid, section_id: Option<Uuid>, title: &str) -> Task {
        Task {
            id,
            project_id,
            title: title.to_string(),
            description: None,
            priority: Priority::Normal,
            section_id,
            position: Position::parse("a0").unwrap(),
            state: TaskState::Open,
            tags: Tags::default(),
            created_at: at(),
            external_identity: None,
            external_metadata: None,
        }
    }

    fn comment(id: Uuid, task_id: Uuid, contents: &str) -> Comment {
        Comment {
            id,
            task_id,
            contents: contents.to_string(),
            created_at: at(),
        }
    }

    fn document(
        project: Project,
        sections: Vec<Section>,
        tasks: Vec<Task>,
        comments: Vec<Comment>,
    ) -> ProjectSnapshot {
        ProjectSnapshot {
            project,
            sections,
            tasks,
            comments,
            automations: Vec::new(),
        }
    }

    fn file(projects: Vec<ProjectSnapshot>) -> SnapshotFile {
        SnapshotFile {
            version: SNAPSHOT_VERSION,
            app_version: String::new(),
            projects,
            last_seen_seq: None,
        }
    }

    fn import_state(
        projects: Vec<Project>,
        sections: Vec<Section>,
        tasks: Vec<Task>,
        comments: Vec<Comment>,
    ) -> ImportState {
        ImportState {
            projects,
            sections,
            tasks,
            comments,
            automations: Vec::new(),
            tombstones: HashSet::new(),
        }
    }

    fn create_count(plan: &ImportPlan) -> usize {
        plan.operations
            .iter()
            .filter(|operation| {
                matches!(
                    &operation.kind,
                    OperationKind::CreateProject { .. }
                        | OperationKind::CreateSection { .. }
                        | OperationKind::CreateTask { .. }
                        | OperationKind::CreateComment { .. }
                        | OperationKind::CreateAutomation { .. }
                )
            })
            .count()
    }

    #[test]
    fn created_tasks_carry_external_fields() {
        let project_id = uuid(1);
        let section_id = uuid(2);
        let task_id = uuid(3);

        let identity = ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 42,
        };
        let metadata = ExternalMetadata::Github {
            title: Some("Widget".to_string()),
            description: Some("A widget".to_string()),
            kind: Some("issue".to_string()),
            state: Some("open".to_string()),
            reason: None,
            draft: Some(false),
            updated_at: Some(at()),
            error: None,
        };

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![section(section_id, project_id, "todo")],
            vec![Task {
                external_identity: Some(identity.clone()),
                external_metadata: Some(metadata.clone()),
                ..task(task_id, project_id, Some(section_id), "task")
            }],
            vec![],
        )]);

        let state = import_state(vec![], vec![], vec![], vec![]);
        let plan = plan_import(&incoming, &state, ImportMode::Merge).unwrap();

        let (created_identity, created_metadata) = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateTask {
                    external_identity,
                    external_metadata,
                    ..
                } => Some((external_identity, external_metadata)),
                _ => None,
            })
            .expect("import should create the task");

        assert_eq!(created_identity.as_ref(), Some(&identity));
        assert_eq!(created_metadata.as_deref(), Some(&metadata));
    }

    #[test]
    fn tombstoned_project_remints_project_and_every_child() {
        let project_id = uuid(1);
        let section_id = uuid(2);
        let task_id = uuid(3);
        let comment_id = uuid(4);

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![section(section_id, project_id, "todo")],
            vec![task(task_id, project_id, Some(section_id), "task")],
            vec![comment(comment_id, task_id, "contents")],
        )]);

        let mut state = import_state(vec![], vec![], vec![], vec![]);
        state.tombstones.insert((EntityType::Project, project_id));

        let plan = plan_import(&incoming, &state, ImportMode::Merge).unwrap();

        assert_eq!(plan.operations.len(), 4);

        let OperationKind::CreateProject {
            id: new_project, ..
        } = &plan.operations[0].kind
        else {
            panic!("expected CreateProject, got {:?}", plan.operations[0].kind);
        };
        assert_ne!(*new_project, project_id);

        let OperationKind::CreateSection {
            id: new_section,
            project_id: section_project,
            ..
        } = &plan.operations[1].kind
        else {
            panic!("expected CreateSection, got {:?}", plan.operations[1].kind);
        };
        assert_ne!(*new_section, section_id);
        assert_eq!(*section_project, *new_project);

        let OperationKind::CreateTask {
            id: new_task,
            project_id: task_project,
            section_id: task_section,
            ..
        } = &plan.operations[2].kind
        else {
            panic!("expected CreateTask, got {:?}", plan.operations[2].kind);
        };
        assert_ne!(*new_task, task_id);
        assert_eq!(*task_project, *new_project);
        assert_eq!(*task_section, Some(*new_section));

        let OperationKind::CreateComment {
            id: new_comment,
            task_id: comment_task,
            ..
        } = &plan.operations[3].kind
        else {
            panic!("expected CreateComment, got {:?}", plan.operations[3].kind);
        };
        assert_ne!(*new_comment, comment_id);
        assert_eq!(*comment_task, *new_task);

        let remaps: Vec<(EntityType, Uuid)> = plan
            .report
            .remapped
            .iter()
            .map(|remap| (remap.entity_type, remap.from))
            .collect();
        assert_eq!(
            remaps,
            vec![
                (EntityType::Project, project_id),
                (EntityType::Section, section_id),
                (EntityType::Task, task_id),
                (EntityType::Comment, comment_id),
            ]
        );

        assert_eq!(plan.report.projects, 1);
        assert_eq!(plan.report.sections, 1);
        assert_eq!(plan.report.tasks, 1);
        assert_eq!(plan.report.comments, 1);
    }

    #[test]
    fn live_project_is_upserted_without_creates() {
        let project_id = uuid(1);
        let section_id = uuid(2);
        let task_id = uuid(3);
        let comment_id = uuid(4);

        let live_state = import_state(
            vec![Project {
                description: Some("old".to_string()),
                ..project(project_id, "p")
            }],
            vec![section(section_id, project_id, "todo")],
            vec![task(task_id, project_id, Some(section_id), "old")],
            vec![comment(comment_id, task_id, "old")],
        );

        let incoming = file(vec![document(
            Project {
                description: Some("new".to_string()),
                ..project(project_id, "p")
            },
            vec![Section {
                name: "renamed".to_string(),
                color: Some(Color::Red),
                ..section(section_id, project_id, "todo")
            }],
            vec![Task {
                title: "new".to_string(),
                position: Position::parse("a1").unwrap(),
                ..task(task_id, project_id, Some(section_id), "old")
            }],
            vec![comment(comment_id, task_id, "new")],
        )]);

        let plan = plan_import(&incoming, &live_state, ImportMode::Merge).unwrap();

        assert_eq!(create_count(&plan), 0);

        let description = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::UpdateProject { description, .. } => Some(description),
                _ => None,
            })
            .expect("expected UpdateProject");
        assert_eq!(description, &Some(Some("new".to_string())));

        let (name, color) = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::UpdateSection { name, color, .. } => Some((name, color)),
                _ => None,
            })
            .expect("expected UpdateSection");
        assert_eq!(name, &Some("renamed".to_string()));
        match color {
            Some(OperationColorChange::Set(Color::Red)) => {}
            other => panic!("expected the section color set to red, got {other:?}"),
        }

        let title = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::UpdateTask { title, .. } => Some(title),
                _ => None,
            })
            .expect("expected UpdateTask");
        assert_eq!(title, &Some("new".to_string()));

        let position = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::MoveTask { position, .. } => Some(position),
                _ => None,
            })
            .expect("expected MoveTask");
        assert_eq!(position, &Position::parse("a1").unwrap());

        let contents = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::UpdateComment { contents, .. } => Some(contents),
                _ => None,
            })
            .expect("expected UpdateComment");
        assert_eq!(contents.as_str(), "new");

        assert!(plan.report.remapped.is_empty());
        assert_eq!(plan.report.projects, 1);
        assert_eq!(plan.report.sections, 1);
        assert_eq!(plan.report.tasks, 1);
        assert_eq!(plan.report.comments, 1);
    }

    #[test]
    fn colliding_project_name_is_renamed() {
        let incoming = file(vec![document(
            project(uuid(20), "alpha"),
            vec![],
            vec![],
            vec![],
        )]);

        let state = import_state(vec![project(uuid(10), "alpha")], vec![], vec![], vec![]);
        let plan = plan_import(&incoming, &state, ImportMode::Merge).unwrap();
        let name = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateProject { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .expect("expected CreateProject");
        assert_eq!(name, "alpha (2)");

        let state = import_state(
            vec![project(uuid(10), "alpha"), project(uuid(11), "alpha (2)")],
            vec![],
            vec![],
            vec![],
        );
        let plan = plan_import(&incoming, &state, ImportMode::Merge).unwrap();
        let name = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateProject { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .expect("expected CreateProject");
        assert_eq!(name, "alpha (3)");
    }

    #[test]
    fn skip_mode_drops_colliding_projects() {
        let incoming = file(vec![document(
            project(uuid(20), "alpha"),
            vec![],
            vec![],
            vec![],
        )]);
        let state = import_state(vec![project(uuid(10), "alpha")], vec![], vec![], vec![]);

        let plan = plan_import(&incoming, &state, ImportMode::Skip).unwrap();

        assert!(plan.operations.is_empty());
        assert_eq!(plan.report.projects, 0);
    }

    #[test]
    fn fail_mode_rejects_colliding_project_names() {
        let incoming = file(vec![document(
            project(uuid(20), "alpha"),
            vec![],
            vec![],
            vec![],
        )]);
        let state = import_state(vec![project(uuid(10), "alpha")], vec![], vec![], vec![]);

        let result = plan_import(&incoming, &state, ImportMode::Fail);

        assert!(matches!(result, Err(StorageError::Conflict(_))));
    }

    #[test]
    fn replace_mode_deletes_then_creates() {
        let live_id = uuid(10);
        let incoming_id = uuid(20);
        let incoming = file(vec![document(
            project(incoming_id, "alpha"),
            vec![],
            vec![],
            vec![],
        )]);
        let state = import_state(vec![project(live_id, "alpha")], vec![], vec![], vec![]);

        let plan = plan_import(&incoming, &state, ImportMode::Replace).unwrap();

        let OperationKind::DeleteProject { id: deleted } = &plan.operations[0].kind else {
            panic!("expected DeleteProject, got {:?}", plan.operations[0].kind);
        };
        assert_eq!(*deleted, live_id);

        let OperationKind::CreateProject { id, name, .. } = &plan.operations[1].kind else {
            panic!("expected CreateProject, got {:?}", plan.operations[1].kind);
        };
        assert_eq!(*id, incoming_id);
        assert_eq!(name, "alpha");
    }

    #[test]
    fn live_section_with_same_name_is_reused() {
        let project_id = uuid(1);
        let live_section_id = uuid(2);
        let incoming_section_id = uuid(3);
        let task_id = uuid(4);

        let live_state = import_state(
            vec![project(project_id, "p")],
            vec![section(live_section_id, project_id, "Todo")],
            vec![],
            vec![],
        );

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![section(incoming_section_id, project_id, "Todo")],
            vec![task(task_id, project_id, Some(incoming_section_id), "task")],
            vec![],
        )]);

        let plan = plan_import(&incoming, &live_state, ImportMode::Merge).unwrap();

        assert!(
            plan.operations
                .iter()
                .all(|operation| !matches!(&operation.kind, OperationKind::CreateSection { .. }))
        );

        let section_id = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateTask { section_id, .. } => Some(section_id),
                _ => None,
            })
            .expect("expected CreateTask");
        assert_eq!(*section_id, Some(live_section_id));

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![Section {
                color: Some(Color::Red),
                ..section(incoming_section_id, project_id, "Todo")
            }],
            vec![task(task_id, project_id, Some(incoming_section_id), "task")],
            vec![],
        )]);

        let plan = plan_import(&incoming, &live_state, ImportMode::Merge).unwrap();

        let (name, color) = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::UpdateSection {
                    id, name, color, ..
                } if *id == live_section_id => Some((name, color)),
                _ => None,
            })
            .expect("expected UpdateSection for the reused section");
        assert!(name.is_none());
        match color {
            Some(OperationColorChange::Set(Color::Red)) => {}
            other => panic!("expected the section color set to red, got {other:?}"),
        }
    }

    #[test]
    fn tombstoned_section_with_same_name_is_reused() {
        let project_id = uuid(1);
        let live_section_id = uuid(2);
        let incoming_section_id = uuid(3);
        let task_id = uuid(4);

        let mut live_state = import_state(
            vec![project(project_id, "p")],
            vec![section(live_section_id, project_id, "Todo")],
            vec![],
            vec![],
        );
        live_state
            .tombstones
            .insert((EntityType::Section, incoming_section_id));

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![section(incoming_section_id, project_id, "Todo")],
            vec![task(task_id, project_id, Some(incoming_section_id), "task")],
            vec![],
        )]);

        let plan = plan_import(&incoming, &live_state, ImportMode::Merge).unwrap();

        assert!(
            plan.operations
                .iter()
                .all(|operation| !matches!(&operation.kind, OperationKind::CreateSection { .. }))
        );
        assert!(
            plan.report
                .remapped
                .iter()
                .all(|remap| remap.from != incoming_section_id)
        );

        let section_id = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateTask { section_id, .. } => Some(section_id),
                _ => None,
            })
            .expect("expected CreateTask");
        assert_eq!(*section_id, Some(live_section_id));
    }

    #[test]
    fn identical_document_plans_no_operations() {
        let project_id = uuid(1);
        let section_id = uuid(2);
        let task_id = uuid(3);
        let comment_id = uuid(4);

        let live_state = import_state(
            vec![project(project_id, "p")],
            vec![section(section_id, project_id, "todo")],
            vec![task(task_id, project_id, Some(section_id), "task")],
            vec![comment(comment_id, task_id, "contents")],
        );

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![section(section_id, project_id, "todo")],
            vec![task(task_id, project_id, Some(section_id), "task")],
            vec![comment(comment_id, task_id, "contents")],
        )]);

        let plan = plan_import(&incoming, &live_state, ImportMode::Merge).unwrap();

        assert!(plan.operations.is_empty());
        assert_eq!(plan.report.projects, 1);
        assert_eq!(plan.report.sections, 1);
        assert_eq!(plan.report.tasks, 1);
        assert_eq!(plan.report.comments, 1);
    }

    #[test]
    fn cross_project_section_id_is_a_conflict() {
        let live_project_id = uuid(1);
        let incoming_project_id = uuid(10);
        let section_id = uuid(2);

        let live_state = import_state(
            vec![project(live_project_id, "a")],
            vec![section(section_id, live_project_id, "Todo")],
            vec![],
            vec![],
        );

        let incoming = file(vec![document(
            project(incoming_project_id, "b"),
            vec![section(section_id, incoming_project_id, "Todo")],
            vec![],
            vec![],
        )]);

        let result = plan_import(&incoming, &live_state, ImportMode::Merge);

        assert!(matches!(result, Err(StorageError::Conflict(_))));
    }

    #[test]
    fn task_section_from_another_project_is_a_conflict() {
        let other_project_id = uuid(1);
        let project_id = uuid(10);
        let section_id = uuid(2);
        let task_id = uuid(3);

        let live_state = import_state(
            vec![project(other_project_id, "other")],
            vec![section(section_id, other_project_id, "Todo")],
            vec![],
            vec![],
        );

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![],
            vec![task(task_id, project_id, Some(section_id), "task")],
            vec![],
        )]);

        let result = plan_import(&incoming, &live_state, ImportMode::Merge);

        assert!(matches!(result, Err(StorageError::Conflict(_))));
    }

    #[test]
    fn comment_task_from_another_project_is_a_conflict() {
        let other_project_id = uuid(1);
        let project_id = uuid(10);
        let task_id = uuid(2);
        let comment_id = uuid(3);

        let live_state = import_state(
            vec![project(other_project_id, "other")],
            vec![],
            vec![task(task_id, other_project_id, None, "task")],
            vec![],
        );

        let incoming = file(vec![document(
            project(project_id, "p"),
            vec![],
            vec![],
            vec![comment(comment_id, task_id, "contents")],
        )]);

        let result = plan_import(&incoming, &live_state, ImportMode::Merge);

        assert!(matches!(result, Err(StorageError::Conflict(_))));
    }

    #[test]
    fn new_project_automation_is_created() {
        let project_id = uuid(1);
        let section_id = uuid(2);
        let automation_id = uuid(3);

        let mut incoming_document = document(
            project(project_id, "p"),
            vec![section(section_id, project_id, "todo")],
            vec![],
            vec![],
        );
        incoming_document.automations.push(Automation {
            id: automation_id,
            project_id,
            trigger: AutomationTrigger::TaskEntersSection { section_id },
            actions: vec![AutomationAction::MoveToSection {
                section_id: Some(section_id),
            }],
            created_at: at(),
        });

        let incoming = file(vec![incoming_document]);
        let state = import_state(vec![], vec![], vec![], vec![]);

        let plan = plan_import(&incoming, &state, ImportMode::Merge).unwrap();

        let (id, created_project_id, trigger, actions) = plan
            .operations
            .iter()
            .find_map(|operation| match &operation.kind {
                OperationKind::CreateAutomation {
                    id,
                    project_id,
                    trigger,
                    actions,
                    ..
                } => Some((id, project_id, trigger, actions)),
                _ => None,
            })
            .expect("expected CreateAutomation");

        assert_eq!(*id, automation_id);
        assert_eq!(*created_project_id, project_id);
        assert_eq!(
            *trigger,
            AutomationTrigger::TaskEntersSection { section_id }
        );
        assert_eq!(
            *actions,
            vec![AutomationAction::MoveToSection {
                section_id: Some(section_id)
            }]
        );
        assert_eq!(plan.report.automations, 1);
    }
}
