mod hints;
pub use hints::Hints;

pub mod popup;
pub use popup::Popup;

mod root;
pub use root::Root;

mod shared;
pub use shared::Button;
pub use shared::InputBlock;

mod task_details;
pub use task_details::TaskDetails;

mod task_line;
pub use task_line::TaskLine;

mod task_list;
pub use task_list::TaskList;

mod task_status_list;
pub use task_status_list::TaskStatusList;

use crate::error::StorageError;
use crate::models::StatusID;
use crate::models::{Project, ProjectID, Status, Task, TaskID};
use crate::store::TaskStore;
use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

/// RenderContext is the context passed to components during
/// rendering, containg the current frame and area to render into.
pub struct RenderContext<'a, 'b> {
    pub state: &'a State,

    pub frame: &'a mut Frame<'b>,
    pub area: Rect,
}

impl<'a, 'b> RenderContext<'a, 'b> {
    pub fn render<W: Widget>(&mut self, widget: W) {
        self.frame.render_widget(widget, self.area);
    }

    pub fn with_area<'c>(&'c mut self, area: Rect) -> RenderContext<'c, 'b> {
        RenderContext {
            state: self.state,
            frame: &mut *self.frame,
            area,
        }
    }

    pub fn render_popup_frame(
        &mut self,
        width_constraint: Constraint,
        height_constraint: Constraint,
        block: Option<Block>,
    ) -> Rect {
        // get the popup area
        let total_area = self.frame.area();
        let popup_area = Self::centered_rect(width_constraint, height_constraint, total_area);

        // use provided or default block
        let block = block.unwrap_or(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain),
        );

        // clear the area behind the popup and render the block
        self.frame.render_widget(Clear, popup_area);
        self.frame.render_widget(block, popup_area);

        popup_area.inner(Margin::new(1, 1))
    }

    fn centered_rect(
        width_constraint: Constraint,
        height_constraint: Constraint,
        total_area: Rect,
    ) -> Rect {
        let [_, content_area, _] =
            Layout::horizontal([Constraint::Fill(1), width_constraint, Constraint::Fill(1)])
                .flex(Flex::Center)
                .areas(total_area);

        let [_, content_area, _] =
            Layout::vertical([Constraint::Fill(1), height_constraint, Constraint::Fill(1)])
                .flex(Flex::Center)
                .areas(content_area);

        content_area
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
    _project: Project,
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
    pub fn next_task(&self, task_id: TaskID) -> Option<&Task> {
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
    pub fn previous_task(&self, task_id: TaskID) -> Option<&Task> {
        let mut tasks = self.task_iterator().rev();

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
        let mut statuses = self.status_iterator();

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
        let mut statuses = self.status_iterator().rev();

        while let Some(status) = statuses.next() {
            if status.id == status_id {
                return statuses.next();
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

    fn status_iterator(&self) -> impl DoubleEndedIterator<Item = &Status> + '_ {
        self.status_tasks.iter().map(|st| &st.status)
    }
}

impl From<&State> for ProjectStatusTasks {
    fn from(state: &State) -> Self {
        Self {
            _project: state.project.clone(),
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
