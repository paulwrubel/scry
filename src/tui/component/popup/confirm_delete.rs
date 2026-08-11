use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};

pub struct ConfirmDelete {
    task_id: i64,
    task_title: String,
    is_confirmation_option_highlighted: bool,
}

impl ConfirmDelete {
    pub fn new(task_id: i64, task_title: String) -> Self {
        Self {
            task_id,
            task_title,
            is_confirmation_option_highlighted: false,
        }
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => Some(Action::DismissPopup),
            KeyCode::Char('y') => Some(Action::DeleteTask(self.task_id)),
            KeyCode::Enter => {
                if self.is_confirmation_option_highlighted {
                    Some(Action::DeleteTask(self.task_id))
                } else {
                    Some(Action::DismissPopup)
                }
            }
            KeyCode::Left | KeyCode::Right => {
                self.is_confirmation_option_highlighted = !self.is_confirmation_option_highlighted;
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let lines = [
            Line::from(format!("  Delete task \"{}\"?", self.task_title)),
            Line::from(""),
        ];

        let yes_style = if self.is_confirmation_option_highlighted {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let no_style = if !self.is_confirmation_option_highlighted {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let button_line = Line::from(vec![
            Span::styled("        ", Style::default()),
            Span::styled("[n]o", no_style),
            Span::styled("   ", Style::default()),
            Span::styled("[y]es", yes_style),
            Span::styled("        ", Style::default()),
        ]);

        let all_lines = vec![lines[0].clone(), lines[1].clone(), button_line];

        let height = 5u16;
        let width = 44u16;
        render_centered_popup(
            ctx.frame,
            all_lines,
            height.min(ctx.frame.area().height),
            width,
            "Confirm",
        );
    }
}

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
