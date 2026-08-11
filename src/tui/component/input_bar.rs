use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct InputBar {
    buffer: String,
    cursor_position: usize,
    focused: bool,
}

impl InputBar {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_position: 0,
            focused: false,
        }
    }

    pub fn focus(&mut self) {
        // always start from a clean slate so stale text can't be submitted
        self.buffer.clear();
        self.cursor_position = 0;
        self.focused = true;
    }

    pub fn blur(&mut self) {
        self.buffer.clear();
        self.cursor_position = 0;
        self.focused = false;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Esc => {
                self.blur();
                None
            }
            KeyCode::Up => {
                self.blur();
                Some(Action::MoveFocusUp)
            }
            KeyCode::Enter => {
                let title = self.buffer.trim().to_string();
                self.blur();
                if title.is_empty() {
                    None
                } else {
                    Some(Action::AddTask(title))
                }
            }
            KeyCode::Char(c) => {
                self.buffer.insert(self.cursor_position, c);
                self.cursor_position += 1;
                None
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.buffer.remove(self.cursor_position);
                }
                None
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                self.cursor_position = (self.cursor_position + 1).min(self.buffer.len());
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let block = if self.focused {
            Block::default().borders(Borders::ALL).title("New Task")
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().add_modifier(Modifier::DIM))
        };

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

        let lines = if self.focused {
            if self.buffer.is_empty() {
                vec![Line::from(Span::raw(" "))]
            } else {
                vec![Line::from(Span::raw(&self.buffer))]
            }
        } else {
            vec![Line::from(Span::styled(
                "Add a task...",
                Style::default().add_modifier(Modifier::DIM),
            ))]
        };

        ctx.render_widget(block);
        ctx.frame
            .render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), text_area);

        // if self.focused {
        let col = if self.buffer.is_empty() {
            text_area.x
        } else {
            text_area.x + self.cursor_position.min(self.buffer.len()) as u16
        };
        ctx.frame.set_cursor_position((col, text_area.y));
        // }
    }
}
