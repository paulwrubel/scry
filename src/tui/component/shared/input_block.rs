use crate::tui::action::Action;
use crate::tui::component::shared::Input;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::KeyEvent;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

pub struct InputBlock {
    pub is_focused: bool,
    title: Option<String>,

    input: Input,
}

impl InputBlock {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,
            title: None,

            input: Input::new(is_focused),
        }
    }

    pub fn with_title(self, title: String) -> Self {
        Self {
            title: Some(title),
            ..self
        }
    }

    pub fn with_placeholder_text(self, placeholder_text: String) -> Self {
        Self {
            input: self.input.with_placeholder_text(placeholder_text),
            ..self
        }
    }

    pub fn with_text(self, text: String) -> Self {
        Self {
            input: self.input.with_text(text),
            ..self
        }
    }

    // pub fn reset(&mut self) {
    //     self.input.reset();
    // }

    pub fn focus(&mut self) {
        self.input.focus();
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.input.blur();
        self.is_focused = false;
    }

    pub fn buffer_text(&self) -> &str {
        self.input.buffer_text()
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        if !self.is_focused {
            return None;
        }
        self.input.handle_event(state, key)
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let style = if self.is_focused {
            Style::default()
        } else {
            Style::default().dim()
        };

        let mut block = Block::default().borders(Borders::ALL).border_style(style);
        if let Some(title) = &self.title {
            block = block.title(title.as_str())
        };

        self.input.render(&mut ctx.with_area(block.inner(ctx.area)));
        ctx.render(&block);
    }
}
