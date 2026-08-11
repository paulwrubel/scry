use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct InputBar {
    pub is_focused: bool,

    buffer: String,
    cursor_position: usize,
}

impl InputBar {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,

            buffer: String::new(),
            cursor_position: 0,
        }
    }

    pub fn focus(&mut self) {
        self.cursor_position = self.buffer.len();
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.is_focused = false;
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        if !self.is_focused {
            return None;
        }
        match key.code {
            KeyCode::Up => Some(Action::MoveFocusUp),
            KeyCode::Down => Some(Action::MoveFocusDown),
            KeyCode::Enter => {
                let title = self.buffer.trim().to_string();
                self.buffer.clear();
                self.cursor_position = 0;
                if title.is_empty() {
                    None
                } else {
                    Some(Action::AddTask(title))
                }
            }
            KeyCode::Char(c) => {
                self.buffer.insert(self.cursor_position, c);
                self.cursor_position += c.len_utf8();
                None
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    let prev = self.buffer.floor_char_boundary(self.cursor_position - 1);
                    self.buffer.remove(prev);
                    self.cursor_position = prev;
                }
                None
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position =
                        self.buffer.floor_char_boundary(self.cursor_position - 1);
                }
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.buffer.len() {
                    self.cursor_position = self.buffer.ceil_char_boundary(self.cursor_position + 1);
                }
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let style = if self.is_focused {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title("New Task");

        // split the block interior vertically so the input sits on the middle row
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(block.inner(ctx.area));

        let text_area = rows[1];

        let content_line = if !self.buffer.is_empty() || self.is_focused {
            Line::from(Span::styled(&self.buffer, style))
        } else {
            Line::from(Span::styled("Add a task...", style))
        };

        ctx.render_widget(block);
        ctx.frame.render_widget(
            Paragraph::new(content_line).wrap(Wrap { trim: false }),
            text_area,
        );
        if self.is_focused {
            let col = if self.buffer.is_empty() {
                text_area.x
            } else {
                text_area.x
                    + self.buffer[..self.cursor_position.min(self.buffer.len())]
                        .chars()
                        .count() as u16
            };
            ctx.frame.set_cursor_position((col, text_area.y));
        }
    }
}
