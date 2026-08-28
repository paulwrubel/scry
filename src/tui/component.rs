mod command_input;
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

use crate::tui::state::ProjectState;
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
