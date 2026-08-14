mod input_bar;
pub use input_bar::InputBar;

pub mod popup;
pub use popup::Popup;

mod root;
pub use root::Root;

mod hint_bar;
pub use hint_bar::HintBar;

mod task_list;
pub use task_list::TaskList;

mod task_status_list;

mod task_line;

use crate::error::StorageError;
use crate::models::{Project, ProjectID, Status, Task, TaskID};
use crate::store::TaskStore;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// RenderContext is the context passed to components during
/// rendering, containg the current frame and area to render into.
pub struct RenderContext<'a, 'b> {
    pub state: &'a State,

    pub frame: &'a mut Frame<'b>,
    pub area: Rect,
}

impl RenderContext<'_, '_> {
    pub fn render_widget<W: Widget>(&mut self, widget: W) {
        self.frame.render_widget(widget, self.area);
    }
}

/// Domain data passed to components each frame, so they have accurate and up-to-date backend info
#[derive(Debug, Clone)]
pub struct State {
    pub project: Project,
    pub statuses: Vec<Status>,
    pub tasks: Vec<Task>,
}

impl State {
    pub async fn load_from_store(
        store: &dyn TaskStore,
        project_id: ProjectID,
    ) -> Result<State, StorageError> {
        let project = store
            .get_project_by_id(project_id)
            .await?
            .ok_or(StorageError::NotFound(format!(
                "Project not found for id {project_id}"
            )))?;
        let statuses = store.list_statuses(project_id).await?;
        let tasks = store.list_tasks(project_id, None).await?;

        Ok(Self {
            project,
            statuses,
            tasks,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StatusTasks {
    status: Status,
    tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct ProjectStatusTasks {
    project: Project,
    status_tasks: Vec<StatusTasks>,
}

impl ProjectStatusTasks {
    pub fn get_task_by_id(&self, task_id: TaskID) -> Option<&Task> {
        self.task_iterator().find(|t| t.id == task_id)
    }
    /// Get the first Task in any status.
    ///
    /// It will return None if there are no tasks.
    pub fn first(&self) -> Option<&Task> {
        self.task_iterator().next()
    }

    /// Get the Task immediately following the one with the provided ID in the order.
    ///
    /// It may return None if there is no following Task.
    pub fn next(&self, task_id: TaskID) -> Option<&Task> {
        let mut tasks = self.task_iterator();

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
    pub fn previous(&self, task_id: TaskID) -> Option<&Task> {
        let mut tasks = self.task_iterator().rev();

        while let Some(task) = tasks.next() {
            if task.id == task_id {
                return tasks.next();
            }
        }

        None
    }

    pub fn index_in_status(&self, task_id: TaskID) -> Option<usize> {
        self.status_tasks
            .iter()
            .flat_map(|status| status.tasks.iter().enumerate())
            .find(|(_, task)| task.id == task_id)
            .map(|(i, _)| i)
    }

    fn task_iterator(&self) -> impl DoubleEndedIterator<Item = &Task> + '_ {
        // flatten into a single ordered stream of tasks
        self.status_tasks
            .iter()
            .flat_map(|status| status.tasks.iter())
    }
}

impl From<&State> for ProjectStatusTasks {
    fn from(state: &State) -> Self {
        Self {
            project: state.project.clone(),
            status_tasks: state
                .statuses
                .iter()
                .map(|status| StatusTasks {
                    status: status.clone(),
                    tasks: state
                        .tasks
                        .iter()
                        .filter(|task| task.status_id == status.id)
                        .cloned()
                        .collect(),
                })
                .collect(),
        }
    }
}
