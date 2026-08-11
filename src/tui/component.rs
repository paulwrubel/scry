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

use crate::error::StorageError;
use crate::models::{Project, ProjectID, Status, Task};
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
