use chrono::DateTime;
use chrono::Utc;

use crate::error::StorageError;
use crate::models::Note;
use crate::models::StatusID;
use crate::models::{Project, ProjectID, Status, Task, TaskID};
use crate::store::TaskStore;

#[derive(Debug, Clone)]
pub struct ProjectState {
    project: Project,
    pub(crate) statuses_with_tasks: Vec<StatusWithTasks>,
}
#[derive(Debug, Clone)]
pub struct StatusWithTasks {
    pub(crate) status: Status,
    pub(crate) is_entry: bool,
    pub(crate) tasks_with_notes: Vec<TaskWithNotes>,
}
#[derive(Debug, Clone)]
pub struct TaskWithNotes {
    pub(crate) id: TaskID,
    pub(crate) project_id: ProjectID,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) status_id: i64,
    pub(crate) position: i32,
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
            status_id: task.status_id,
            position: task.position,
            created_at: task.created_at,
            notes: notes.into_iter().collect(),
        }
    }
}

impl From<&TaskWithNotes> for Task {
    fn from(value: &TaskWithNotes) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            title: value.title.clone(),
            description: value.description.clone(),
            status_id: value.status_id,
            position: value.position,
            created_at: value.created_at,
        }
    }
}

impl ProjectState {
    pub async fn load_from_store(
        store: &dyn TaskStore,
        project_id: ProjectID,
    ) -> Result<Self, StorageError> {
        let project = store
            .get_project_by_id(project_id)
            .await?
            .ok_or(StorageError::NotFound(format!(
                "Project not found for id {project_id}"
            )))?;
        let statuses = store.get_all_statuses_by_project_id(project_id).await?;
        let tasks = store.get_all_tasks_by_project_id(project_id).await?;
        let notes = store.get_all_notes_by_project_id(project_id).await?;

        Ok(Self {
            project: project.clone(),
            statuses_with_tasks: statuses
                .iter()
                .map(|status| StatusWithTasks {
                    status: status.clone(),
                    is_entry: project.entry_status_id == Some(status.id),
                    tasks_with_notes: tasks
                        .iter()
                        .filter(|task| task.status_id == status.id)
                        .map(|task| {
                            TaskWithNotes::new(
                                task,
                                notes.iter().filter(|note| note.task_id == task.id).cloned(),
                            )
                        })
                        .collect(),
                })
                .collect(),
        })
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn get_task_by_id(&self, task_id: TaskID) -> Option<&TaskWithNotes> {
        self.tasks().find(|t| t.id == task_id)
    }

    #[allow(dead_code)]
    pub fn get_status_by_id(&self, status_id: StatusID) -> Option<&Status> {
        self.statuses().find(|s| s.id == status_id)
    }

    pub fn get_status_by_name(&self, status_name: &str) -> Option<&Status> {
        self.statuses().find(|s| s.name == status_name)
    }

    pub fn tasks_in_status(&self, status_id: StatusID) -> Vec<&TaskWithNotes> {
        self.statuses_with_tasks
            .iter()
            .find_map(|st| {
                if st.status.id == status_id {
                    Some(st.tasks_with_notes.iter().collect())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Get the first Task in any status.
    ///
    /// It will return None if there are no tasks.
    pub fn first(&self) -> Option<&TaskWithNotes> {
        self.tasks().next()
    }

    /// Get the last Task in any status.
    ///
    /// It will return None if there are no tasks.
    pub fn last(&self) -> Option<&TaskWithNotes> {
        self.tasks().next_back()
    }

    /// Get the Task immediately following the one with the provided ID in the order.
    ///
    /// It may return None if there is no following Task.
    pub fn next_task(&self, task_id: TaskID) -> Option<&TaskWithNotes> {
        let mut tasks = self.tasks();

        while let Some(task) = tasks.next() {
            if task.id == task_id {
                return tasks.next();
            }
        }

        None
    }

    /// Get the Task immediately preceding the one with the provided ID in the order.
    ///
    /// It may return None if there is no preceding Task.
    pub fn previous_task(&self, task_id: TaskID) -> Option<&TaskWithNotes> {
        let mut tasks = self.tasks().rev();

        while let Some(task) = tasks.next() {
            if task.id == task_id {
                return tasks.next();
            }
        }

        None
    }

    /// Get the Status immediately following the one with the provided ID in the order.
    ///
    /// It may return None if there is no following Status.
    pub fn next_status(&self, status_id: StatusID) -> Option<&Status> {
        let mut statuses = self.statuses();

        while let Some(status) = statuses.next() {
            if status.id == status_id {
                return statuses.next();
            }
        }

        None
    }

    /// Get the Status immediately preceding the one with the provided ID in the order.
    ///
    /// It may return None if there is no preceding Status.
    pub fn previous_status(&self, status_id: StatusID) -> Option<&Status> {
        let mut statuses = self.statuses().rev();

        while let Some(status) = statuses.next() {
            if status.id == status_id {
                return statuses.next();
            }
        }

        None
    }

    pub fn index_in_status(&self, task_id: TaskID) -> Option<usize> {
        self.statuses_with_tasks
            .iter()
            .flat_map(|status| status.tasks_with_notes.iter().enumerate())
            .find(|(_, task)| task.id == task_id)
            .map(|(i, _)| i)
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
