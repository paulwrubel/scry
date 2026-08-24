use crate::tui::component::RenderContext;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub struct Button {
    pub is_focused: bool,
    text: String,
}

impl Button {
    pub fn new(is_focused: bool, text: String) -> Self {
        Self { is_focused, text }
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.is_focused = false;
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let style = if self.is_focused {
            Style::default().reversed()
        } else {
            Style::default()
        };

        ctx.render(Line::from(Span::styled(&self.text, style)).centered());
    }
}
