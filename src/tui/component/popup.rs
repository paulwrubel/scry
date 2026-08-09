mod confirm_delete;
pub use confirm_delete::ConfirmDelete;

mod project_settings;

pub use project_settings::ProjectSettings;

mod state_picker;
pub use state_picker::StatePicker;

mod task_detail;
pub use task_detail::TaskDetail;

use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::tui::Action;
use crate::tui::component::AppContext;
use crate::tui::component::Component;

pub enum Popup {
    TaskDetail(TaskDetail),
    StatePicker(StatePicker),
    ConfirmDelete(ConfirmDelete),
    ProjectSettings(ProjectSettings),
}

impl Component for Popup {
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action> {
        match self {
            Popup::TaskDetail(p) => p.handle_event(ctx, key),
            Popup::StatePicker(p) => p.handle_event(ctx, key),
            Popup::ConfirmDelete(p) => p.handle_event(ctx, key),
            Popup::ProjectSettings(p) => p.handle_event(ctx, key),
        }
    }

    fn render(&self, ctx: &AppContext, frame: &mut Frame, area: Rect) {
        match self {
            Popup::TaskDetail(p) => p.render(ctx, frame, area),
            Popup::StatePicker(p) => p.render(ctx, frame, area),
            Popup::ConfirmDelete(p) => p.render(ctx, frame, area),
            Popup::ProjectSettings(p) => p.render(ctx, frame, area),
        }
    }
}
