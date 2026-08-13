mod confirm_delete;
pub use confirm_delete::ConfirmDelete;

mod status_selection;
pub use status_selection::StatusSelection;

mod task_detail;
pub use task_detail::TaskDetail;

use crate::tui::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

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

pub(crate) fn render_centered_popup(
    frame: &mut Frame,
    lines: Vec<Line>,
    height: u16,
    width: u16,
    title: &str,
) -> Rect {
    let area = frame.area();
    let popup_area = centered_rect(width, height, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(title);

    // clear the area behind the popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(popup_area)[1];

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner)[1];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    popup_area
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
            Constraint::Length(percent_y),
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
            Constraint::Length(percent_x),
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
        ])
        .split(popup_layout[1])[1]
}
