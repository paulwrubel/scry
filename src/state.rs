use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;

use scry_core::error::StorageError;
use scry_core::models::Note;
use scry_core::models::Priority;
use scry_core::models::StatusId;
use scry_core::models::StatusStyle;
use scry_core::models::Tags;
use scry_core::models::TaskSortingMode;
use scry_core::models::{Project, ProjectId, Status, Task, TaskId};
use scry_core::store::Store;

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
    pub(crate) is_entry: bool,
    pub(crate) tasks_with_notes: Vec<TaskWithNotes>,
    /// Number of tasks removed from the visible set because the status style is
    /// `Hidden`; kept so the status header can still report the true count.
    pub(crate) hidden_task_count: usize,
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
                        is_entry: project.entry_status_id == Some(status.id),
                        tasks_with_notes: subtasks,
                        hidden_task_count: 0,
                    }
                })
                .collect(),
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
                        is_entry: swt.is_entry,
                        tasks_with_notes: filtered_tasks,
                        hidden_task_count: 0,
                    }
                })
                .collect(),
        }
    }

    /// Collapse hidden-styled statuses for the TUI: move each hidden status's tasks
    /// out of the visible task set and record how many were removed, so the status
    /// header can still report the true count. Only the TUI calls this, so CLI
    /// output keeps showing tasks in hidden statuses.
    pub fn with_hidden_statuses_collapsed(self) -> Self {
        Self {
            project: self.project,
            statuses_with_tasks: self
                .statuses_with_tasks
                .into_iter()
                .map(|mut status_with_tasks| {
                    if status_with_tasks.status.style == StatusStyle::Hidden {
                        status_with_tasks.hidden_task_count =
                            status_with_tasks.tasks_with_notes.len();
                        status_with_tasks.tasks_with_notes = Vec::new();
                    }
                    status_with_tasks
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

    pub fn get_status_by_name(&self, status_name: &str) -> Option<&Status> {
        self.statuses().find(|s| s.name == status_name)
    }

    pub fn tasks_in_status(&self, status_id: StatusId) -> Vec<&TaskWithNotes> {
        self.statuses_with_tasks
            .iter()
            .find(|st| st.status.id == status_id)
            .map(|st| st.tasks_with_notes.iter().collect())
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
    pub fn next_task(&self, task_id: TaskId) -> Option<&TaskWithNotes> {
        let mut tasks = self.tasks();

        tasks.find(|task| task.id == task_id)?;
        tasks.next()
    }

    /// Get the Task immediately preceding the one with the provided ID in the order.
    ///
    /// It may return None if there is no preceding Task.
    pub fn previous_task(&self, task_id: TaskId) -> Option<&TaskWithNotes> {
        let mut tasks = self.tasks().rev();

        tasks.find(|task| task.id == task_id)?;
        tasks.next()
    }

    /// Get the Status immediately following the one with the provided ID in the order,
    /// skipping statuses styled `Hidden`. The provided status is assumed to be
    /// non-hidden, since tasks in hidden statuses are never selectable in the TUI.
    ///
    /// It may return None if there is no following Status.
    pub fn next_status(&self, status_id: StatusId) -> Option<&Status> {
        let mut statuses = self
            .statuses()
            .filter(|status| status.style != StatusStyle::Hidden);

        statuses.find(|status| status.id == status_id)?;
        statuses.next()
    }

    /// Get the Status immediately preceding the one with the provided ID in the order,
    /// skipping statuses styled `Hidden`. The provided status is assumed to be
    /// non-hidden, since tasks in hidden statuses are never selectable in the TUI.
    ///
    /// It may return None if there is no preceding Status.
    pub fn previous_status(&self, status_id: StatusId) -> Option<&Status> {
        let mut statuses = self
            .statuses()
            .rev()
            .filter(|status| status.style != StatusStyle::Hidden);

        statuses.find(|status| status.id == status_id)?;
        statuses.next()
    }

    pub fn index_in_status(&self, task_id: TaskId) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use scry_core::models::{Project, StatusStyle, Tags, TaskSortingMode};
    use chrono::Utc;

    fn task(id: TaskId, status_id: StatusId) -> TaskWithNotes {
        TaskWithNotes {
            id,
            project_id: 1,
            title: format!("task {id}"),
            description: None,
            priority: Priority::default(),
            status_id,
            position: id as i32,
            tags: Tags::default(),
            created_at: Utc::now(),
            notes: Vec::new(),
        }
    }

    fn status(
        id: StatusId,
        name: &str,
        style: StatusStyle,
        tasks: Vec<TaskWithNotes>,
    ) -> StatusWithTasks {
        StatusWithTasks {
            status: Status {
                id,
                project_id: 1,
                name: name.to_string(),
                position: id as i32,
                color: None,
                style,
            },
            is_entry: false,
            tasks_with_notes: tasks,
            hidden_task_count: 0,
        }
    }

    fn project_state(statuses: Vec<StatusWithTasks>) -> ProjectState {
        ProjectState {
            project: Project {
                id: 1,
                name: "test".to_string(),
                entry_status_id: None,
                task_sorting_mode: TaskSortingMode::Manual,
                show_priority: false,
                created_at: Utc::now(),
            },
            statuses_with_tasks: statuses,
        }
    }

    #[test]
    fn collapse_moves_hidden_status_tasks_and_records_count() {
        let state = project_state(vec![
            status(1, "todo", StatusStyle::None, vec![task(1, 1), task(2, 1)]),
            status(
                2,
                "archive",
                StatusStyle::Hidden,
                vec![task(3, 2), task(4, 2), task(5, 2)],
            ),
        ]);

        let collapsed = state.with_hidden_statuses_collapsed();

        let visible = &collapsed.statuses_with_tasks[0];
        assert_eq!(visible.tasks_with_notes.len(), 2);
        assert_eq!(visible.hidden_task_count, 0);

        let hidden = &collapsed.statuses_with_tasks[1];
        assert!(hidden.tasks_with_notes.is_empty());
        assert_eq!(hidden.hidden_task_count, 3);

        assert_eq!(collapsed.tasks().count(), 2);
    }

    #[test]
    fn next_status_skips_hidden() {
        let state = project_state(vec![
            status(1, "todo", StatusStyle::None, vec![]),
            status(2, "archive", StatusStyle::Hidden, vec![]),
            status(3, "done", StatusStyle::None, vec![]),
        ]);

        assert_eq!(state.next_status(1).map(|s| s.id), Some(3));
        assert_eq!(state.previous_status(3).map(|s| s.id), Some(1));
    }
}
