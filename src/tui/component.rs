mod command_input;
use chrono::DateTime;
use chrono::Utc;
pub use command_input::CommandInput;

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
use crate::models::Note;
use crate::models::StatusID;
use crate::models::{Project, ProjectID, Status, Task, TaskID};
use crate::store::TaskStore;
use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

/// RenderContext is the context passed to components during
/// rendering, containg the current frame and area to render into.
pub struct RenderContext<'a, 'b> {
    pub state: &'a ProjectState,

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

#[derive(Debug, Clone)]
pub struct ProjectState {
    project: Project,
    statuses_with_tasks: Vec<StatusWithTasks>,
}
#[derive(Debug, Clone)]
pub struct StatusWithTasks {
    status: Status,
    tasks_with_notes: Vec<TaskWithNotes>,
}
#[derive(Debug, Clone)]
pub struct TaskWithNotes {
    id: TaskID,
    project_id: ProjectID,
    title: String,
    description: Option<String>,
    status_id: i64,
    position: i32,
    created_at: DateTime<Utc>,
    notes: Vec<Note>,
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
