use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::tui::action::Action;
use crate::tui::component::{AppContext, Component};

pub struct StatePicker {
    // ── internal ──
    task_id: i64,
    selected_state_index: usize,
    state_count: usize,
    current_state_id: Option<i64>, // the task's current state, for "(current)" label
}

impl StatePicker {
    pub fn new(task_id: i64, state_count: usize, current_state_id: Option<i64>) -> Self {
        StatePicker {
            task_id,
            selected_state_index: current_state_id.unwrap_or(0) as usize, // default to first state if no current state
            state_count,
            current_state_id,
        }
    }

    // the coordinator calls this before handle_event to keep state_count
    // in sync with the domain snapshot
    pub fn sync(&mut self, ctx: &AppContext) {
        self.state_count = ctx.states.len();
        if self.selected_state_index >= self.state_count {
            self.selected_state_index = self.state_count.saturating_sub(1);
        }
    }
}

impl Component for StatePicker {
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::DismissPopup),
            KeyCode::Up => {
                self.selected_state_index = self.selected_state_index.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                self.selected_state_index = self
                    .selected_state_index
                    .saturating_add(1)
                    .min(self.state_count.saturating_sub(1));
                None
            }
            KeyCode::Enter => {
                if let Some(state) = ctx.states.get(self.selected_state_index) {
                    Some(Action::MoveTask {
                        task_id: self.task_id,
                        state_name: state.name.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn render(&self, ctx: &AppContext, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for (i, state) in ctx.states.iter().enumerate() {
            let marker = if i == self.selected_state_index {
                ">"
            } else {
                " "
            };
            let is_current = self.current_state_id == Some(state.id);
            let suffix = if is_current { " (current)" } else { "" };
            let text = format!("  {} {}{}", marker, state.name, suffix);

            let style = if i == self.selected_state_index {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }
        if lines.is_empty() {
            lines.push(Line::from("  (no states)"));
        }

        let height = (lines.len() + 2) as u16;
        let width = 40u16;
        render_centered_popup(
            frame,
            lines,
            height.min(area.height),
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
