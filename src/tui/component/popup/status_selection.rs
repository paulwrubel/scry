use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

pub struct StatusSelection {
    task_id: i64,
    selected_status_index: usize,
    status_count: usize,
    current_status_id: Option<i64>, // the task's current status, for "(current)" label
}

impl StatusSelection {
    pub fn new(task_id: i64, status_count: usize, current_status_id: Option<i64>) -> Self {
        StatusSelection {
            task_id,
            selected_status_index: current_status_id.unwrap_or(0) as usize, // default to first status if no current status
            status_count,
            current_status_id,
        }
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::DismissPopup),
            KeyCode::Up => {
                self.selected_status_index = self.selected_status_index.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                self.selected_status_index = self
                    .selected_status_index
                    .saturating_add(1)
                    .min(self.status_count.saturating_sub(1));
                None
            }
            KeyCode::Enter => {
                if let Some(status) = state.statuses.get(self.selected_status_index) {
                    Some(Action::MoveTask {
                        task_id: self.task_id,
                        status_name: status.name.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let mut lines: Vec<Line> = Vec::new();
        for (i, status) in ctx.state.statuses.iter().enumerate() {
            let marker = if i == self.selected_status_index {
                ">"
            } else {
                " "
            };
            let is_current = self.current_status_id == Some(status.id);
            let suffix = if is_current { " (current)" } else { "" };
            let text = format!("  {} {}{}", marker, status.name, suffix);

            let style = if i == self.selected_status_index {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }
        if lines.is_empty() {
            lines.push(Line::from("  (no statuses)"));
        }

        let height = (lines.len() + 2) as u16;
        let width = 40u16;
        render_centered_popup(
            ctx.frame,
            lines,
            height.min(ctx.area.height),
            width,
            "Move task to...",
        );
    }
}

// duplicated from src/tui/view/popup.rs; task 10 will consolidate these helpers
fn render_centered_popup(
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
