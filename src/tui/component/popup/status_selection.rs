use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State, popup};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

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
        popup::render_centered_popup(
            ctx.frame,
            lines,
            height.min(ctx.area.height),
            width,
            "Move task to...",
        );
    }
}
