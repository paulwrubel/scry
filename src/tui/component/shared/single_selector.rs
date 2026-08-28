use crate::tui::{component::RenderContext, state::ProjectState};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    style::Style,
    text::{Line, Span},
};
use std::fmt::Display;

pub struct SingleSelector<T: Display> {
    pub is_focused: bool,

    options: Vec<T>,
    selected_index: usize,
    item_style_fn: Box<dyn Fn(&T) -> Style>,
}

impl<T: Display> SingleSelector<T> {
    pub fn new_with_item_style<F>(
        is_focused: bool,
        options: Vec<T>,
        item_style_fn: F,
    ) -> Result<Self, &'static str>
    where
        F: Fn(&T) -> Style + 'static,
    {
        if options.is_empty() {
            return Err("Options must not be empty");
        }

        Ok(Self {
            is_focused,
            options,
            selected_index: 0,
            item_style_fn: Box::new(item_style_fn),
        })
    }

    pub fn with_selected_index(self, selected_index: usize) -> Result<Self, &'static str> {
        if selected_index < self.options.len() {
            Ok(Self {
                selected_index,
                ..self
            })
        } else {
            Err("Index out of range of options")
        }
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.is_focused = false;
    }
    pub fn handle_event(&mut self, _state: &ProjectState, key: KeyEvent) {
        if !self.is_focused {
            return;
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Left) => {
                self.selected_index = self
                    .selected_index
                    .checked_sub(1)
                    .unwrap_or(self.options.len() - 1)
            }
            (_, KeyCode::Right) => {
                self.selected_index = (self.selected_index + 1) % self.options.len()
            }
            _ => {}
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let style = if self.is_focused {
            Style::default()
        } else {
            Style::default().dim()
        };

        let selected = &self.options[self.selected_index];

        ctx.render(
            Line::from(vec![
                Span::from("< "),
                Span::styled(selected.to_string(), (self.item_style_fn)(selected)),
                Span::from(" >"),
            ])
            .style(style)
            .centered(),
        );
    }
}
