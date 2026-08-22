use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct InputBlock {
    pub is_focused: bool,
    title: String,
    placeholder_text: Option<String>,

    buffer: String,
    cursor_position: usize,
}

impl InputBlock {
    pub fn new(title: String, is_focused: bool, placeholder_text: Option<String>) -> Self {
        Self {
            is_focused,
            title,
            placeholder_text,

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

    pub fn buffer_text(&self) -> &str {
        &self.buffer
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        if !self.is_focused {
            return None;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.buffer.insert(self.cursor_position, c);
                self.cursor_position += c.len_utf8();
                None
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                if self.cursor_position > 0 {
                    let prev = self.buffer.floor_char_boundary(self.cursor_position - 1);
                    self.buffer.remove(prev);
                    self.cursor_position = prev;
                }
                None
            }
            (_, KeyCode::Left) => {
                if self.cursor_position > 0 {
                    self.cursor_position =
                        self.buffer.floor_char_boundary(self.cursor_position - 1);
                }
                None
            }
            (_, KeyCode::Right) => {
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
            Style::default().dim()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(self.title.as_str());

        let content_area = block.inner(ctx.area);

        let (display_text, display_text_style) = if !self.buffer.is_empty() || self.is_focused {
            (self.buffer.as_str(), style)
        } else {
            (
                self.placeholder_text.as_deref().unwrap_or(""),
                style.italic(),
            )
        };

        let display_text_line = Line::from(Span::styled(display_text, display_text_style));

        ctx.render(&block);
        ctx.with_area(block.inner(ctx.area))
            .render(Paragraph::new(display_text_line).wrap(Wrap { trim: false }));

        if self.is_focused {
            let prefix = &self.buffer[..self.cursor_position];
            let col = content_area.x + Span::raw(prefix).width() as u16;
            ctx.frame.set_cursor_position((col, content_area.y));
        }
    }
}
