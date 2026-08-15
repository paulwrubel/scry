use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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

        // let height = 5u16;
        // let width = 44u16;
        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            Some(Block::default().borders(Borders::ALL).title("Confirm")),
        );

        ctx.frame.render_widget(
            Paragraph::new(all_lines).wrap(Wrap { trim: false }),
            content_area,
        );
    }
}
