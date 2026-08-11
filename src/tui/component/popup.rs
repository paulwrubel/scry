mod confirm_delete;
pub use confirm_delete::ConfirmDelete;

mod status_selection;
pub use status_selection::StatusSelection;

mod task_detail;
pub use task_detail::TaskDetail;

use crate::tui::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::KeyEvent;

pub enum Popup {
    TaskDetail(TaskDetail),
    StatusSelection(StatusSelection),
    ConfirmDelete(ConfirmDelete),
}

impl Popup {
    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        match self {
            Popup::TaskDetail(p) => p.handle_event(state, key),
            Popup::StatusSelection(p) => p.handle_event(state, key),
            Popup::ConfirmDelete(p) => p.handle_event(state, key),
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        match self {
            Popup::TaskDetail(p) => p.render(ctx),
            Popup::StatusSelection(p) => p.render(ctx),
            Popup::ConfirmDelete(p) => p.render(ctx),
        }
    }
}
