mod input_bar;
pub use input_bar::InputBar;

pub mod popup;
pub use popup::Popup;

mod root;
pub use root::Root;

mod status_bar;
pub use status_bar::StatusBar;

mod task_list;
pub use task_list::TaskList;

use crate::models::{Project, State, Task};
use crate::tui::action::Action;
use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

/// Domain data passed to components each frame.
/// References into the coordinator's fields — no cloning.
pub struct AppContext<'a> {
    pub project: &'a Project,
    pub states: &'a [State],
    pub tasks: &'a [Task],
}

/// Core trait for all UI components in the application.
/// Each component owns its UI state, handles its own events,
/// and knows how to render itself into a given area.
pub trait Component {
    /// Handle a key event. Returns Some(Action) if the event produces a
    /// cross-cutting action the parent coordinator needs to process.
    /// Returns None if the event was handled internally (cursor movement,
    /// scrolling, text editing) or ignored.
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action>;

    /// Render the component into the given area of the frame.
    fn render(&self, ctx: &AppContext, frame: &mut Frame, area: Rect);
}
