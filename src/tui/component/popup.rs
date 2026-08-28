mod add_note;
pub use add_note::AddNote;

mod add_or_edit_task;
pub use add_or_edit_task::AddOrEditTask;

mod confirm_delete;
pub use confirm_delete::{ConfirmDelete, ConfirmDeleteEntity};

mod error_info;
pub use error_info::ErrorInfo;

use crate::tui::Action;
use crate::tui::component::{ProjectState, RenderContext};
use ratatui::crossterm::event::KeyEvent;

pub enum Popup {
    AddNote(Box<AddNote>),
    AddOrEditTask(Box<AddOrEditTask>),
    ConfirmDelete(ConfirmDelete),
    ErrorInfo(ErrorInfo),
}

impl Popup {
    pub fn handle_event(&mut self, state: &ProjectState, key: KeyEvent) -> Option<Action> {
        match self {
            Popup::AddNote(p) => p.handle_event(state, key),
            Popup::AddOrEditTask(p) => p.handle_event(state, key),
            Popup::ConfirmDelete(p) => p.handle_event(state, key),
            Popup::ErrorInfo(p) => p.handle_event(state, key),
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        match self {
            Popup::AddNote(p) => p.render(ctx),
            Popup::AddOrEditTask(p) => p.render(ctx),
            Popup::ConfirmDelete(p) => p.render(ctx),
            Popup::ErrorInfo(p) => p.render(ctx),
        }
    }
}
