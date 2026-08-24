use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

pub struct Input {
    pub is_focused: bool,
    placeholder_text: Option<String>,
    prefix_text: String,
    prefix_style: Style,

    buffer: String,
    cursor_position: usize,
}

impl Input {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,
            placeholder_text: None,
            prefix_text: String::new(),
            prefix_style: Style::default(),

            buffer: String::new(),
            cursor_position: 0,
        }
    }

    pub fn with_placeholder_text(self, placeholder_text: String) -> Self {
        Self {
            placeholder_text: Some(placeholder_text),
            ..self
        }
    }

    pub fn with_prefix_text(self, text: String, style: Style) -> Self {
        Self {
            prefix_text: text,
            prefix_style: style,
            ..self
        }
    }

    // pub fn with_text(self, text: String) -> Self {
    //     let length = text.len();

    //     Self {
    //         buffer: text,
    //         cursor_position: length,
    //         ..self
    //     }
    // }

    // pub fn reset(&mut self) {
    //     self.buffer = String::new();
    //     self.cursor_position = 0;
    // }

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

        let prefix_span = Span::styled(&self.prefix_text, self.prefix_style);
        let span = if !self.buffer.is_empty() || self.is_focused {
            Span::styled(self.buffer.as_str(), style)
        } else {
            Span::styled(
                self.placeholder_text.as_deref().unwrap_or(""),
                style.italic(),
            )
        };

        ctx.render(
            Paragraph::new(Line::from(vec![prefix_span.clone(), span])).wrap(Wrap { trim: false }),
        );

        if self.is_focused {
            let prefix = &self.buffer[..self.cursor_position];
            let col = ctx.area.x + prefix_span.width() as u16 + Span::raw(prefix).width() as u16;
            ctx.frame.set_cursor_position((col, ctx.area.y));
        }
    }
}
