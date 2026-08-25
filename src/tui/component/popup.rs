mod confirm_delete;
pub use confirm_delete::{ConfirmDelete, ConfirmDeleteEntity};

mod create_task;
pub use create_task::CreateTask;

mod error_info;
pub use error_info::ErrorInfo;

use crate::tui::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::KeyEvent;

pub enum Popup {
    ConfirmDelete(ConfirmDelete),
    CreateTask(CreateTask),
    ErrorInfo(ErrorInfo),
}

impl Popup {
    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        match self {
            Popup::ConfirmDelete(p) => p.handle_event(state, key),
            Popup::CreateTask(p) => p.handle_event(state, key),
            Popup::ErrorInfo(p) => p.handle_event(state, key),
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        match self {
            Popup::ConfirmDelete(p) => p.render(ctx),
            Popup::CreateTask(p) => p.render(ctx),
            Popup::ErrorInfo(p) => p.render(ctx),
        }
    }
}
