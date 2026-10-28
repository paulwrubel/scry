use chrono::DateTime;
use chrono::Utc;
use scry_core::error::StorageError;
use scry_core::models::{
    Note, Priority, Project, ProjectId, Status, StatusId, Tags, Task, TaskId, TaskSortingMode,
};
use scry_core::store::{ProjectSnapshot, Store};
use serde::Serialize;

// Aug 9, 2026 at 4:40pm
pub const DATETIME_FORMAT_STR: &str = "%b %-d, %Y at %-I:%M%P";

#[derive(Debug, Clone)]
pub struct ProjectState {
    project: Project,
    pub(crate) statuses_with_tasks: Vec<StatusWithTasks>,
}
#[derive(Debug, Clone)]
pub struct StatusWithTasks {
    pub(crate) status: Status,
    pub(crate) tasks_with_notes: Vec<TaskWithNotes>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskWithNotes {
    pub(crate) id: TaskId,
    pub(crate) project_id: ProjectId,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) priority: Priority,
    pub(crate) status_id: i64,
    pub(crate) position: i32,
    pub(crate) tags: Tags,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) notes: Vec<Note>,
}

impl TaskWithNotes {
    pub fn new(task: &Task, notes: impl IntoIterator<Item = Note>) -> Self {
        Self {
            id: task.id,
            project_id: task.project_id,
            title: task.title.clone(),
            description: task.description.clone(),
            priority: task.priority,
            status_id: task.status_id,
            position: task.position,
            tags: task.tags.clone(),
            created_at: task.created_at,
            notes: notes.into_iter().collect(),
        }
    }
}

impl From<TaskWithNotes> for Task {
    fn from(value: TaskWithNotes) -> Self {
        (&value).into()
    }
}

impl From<&TaskWithNotes> for Task {
    fn from(value: &TaskWithNotes) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            title: value.title.clone(),
            description: value.description.clone(),
            priority: value.priority,
            status_id: value.status_id,
            position: value.position,
            tags: value.tags.clone(),
            created_at: value.created_at,
        }
    }
}

impl ProjectState {
    pub async fn load_from_store(
        store: &dyn Store,
        project_id: ProjectId,
    ) -> Result<Self, StorageError> {
        let ProjectSnapshot {
            project,
            statuses,
            tasks,
            notes,
        } = store.get_project_snapshot(project_id).await?;

        Ok(Self {
            statuses_with_tasks: statuses
                .iter()
                .map(|status| {
                    let mut subtasks: Vec<_> = tasks
                        .iter()
                        .filter(|task| task.status_id == status.id)
                        .map(|task| {
                            TaskWithNotes::new(
                                task,
                                notes.iter().filter(|note| note.task_id == task.id).cloned(),
                            )
                        })
                        .collect();
                    subtasks.sort_by(|a, b| match project.task_sorting_mode {
                        TaskSortingMode::Alphabetical => a.title.cmp(&b.title),
                        TaskSortingMode::AlphabeticalCaseInsensitive => {
                            a.title.to_lowercase().cmp(&b.title.to_lowercase())
                        }
                        TaskSortingMode::Id => a.id.cmp(&b.id),
                        TaskSortingMode::Manual => a.position.cmp(&b.position),
                        TaskSortingMode::Priority => a.priority.cmp(&b.priority),
                    });
                    StatusWithTasks {
                        status: status.clone(),
                        tasks_with_notes: subtasks,
                    }
                })
                .collect(),
            project,
        })
    }

    pub fn with_substring_filter(self, filter: String) -> Self {
        Self {
            project: self.project,
            statuses_with_tasks: self
                .statuses_with_tasks
                .into_iter()
                .map(|swt| {
                    let filtered_tasks: Vec<TaskWithNotes> =
                        swt.tasks_with_notes
                            .into_iter()
                            .filter(|task| {
                                task.title.to_lowercase().contains(&filter.to_lowercase())
                                    || task.tags.iter().any(|tag| {
                                        tag.to_lowercase().contains(&filter.to_lowercase())
                                    })
                            })
                            .collect();

                    StatusWithTasks {
                        status: swt.status,
                        tasks_with_notes: filtered_tasks,
                    }
                })
                .collect(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn get_task_by_id(&self, task_id: TaskId) -> Option<&TaskWithNotes> {
        self.tasks().find(|t| t.id == task_id)
    }

    pub fn get_status_by_id(&self, status_id: StatusId) -> Option<&Status> {
        self.statuses().find(|s| s.id == status_id)
    }

    pub fn tasks_in_status(&self, status_id: StatusId) -> Vec<&TaskWithNotes> {
        self.statuses_with_tasks
            .iter()
            .find(|st| st.status.id == status_id)
            .map(|st| st.tasks_with_notes.iter().collect())
            .unwrap_or_default()
    }

    pub fn tasks(&self) -> impl DoubleEndedIterator<Item = &TaskWithNotes> + '_ {
        // flatten into a single ordered stream of tasks
        self.statuses_with_tasks
            .iter()
            .flat_map(|status| status.tasks_with_notes.iter())
    }

    pub fn statuses(&self) -> impl DoubleEndedIterator<Item = &Status> + '_ {
        self.statuses_with_tasks.iter().map(|st| &st.status)
    }
}
