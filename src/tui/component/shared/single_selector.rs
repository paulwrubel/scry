use crate::tui::{component::RenderContext, state::ProjectState};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    style::Style,
    text::{Line, Span},
};
use std::fmt::Display;

pub struct SingleSelectorItem<V, L: Display> {
    pub(crate) value: V,
    pub(crate) label: L,
    pub(crate) label_style: Style,
}

pub struct SingleSelector<V, L: Display> {
    pub is_focused: bool,

    options: Vec<SingleSelectorItem<V, L>>,
    selected_index: usize,
}

impl<V, L: Display> SingleSelector<V, L> {
    pub fn new(
        is_focused: bool,
        options: Vec<SingleSelectorItem<V, L>>,
    ) -> Result<Self, &'static str> {
        if options.is_empty() {
            return Err("Options must not be empty");
        }

        Ok(Self {
            is_focused,

            options,
            selected_index: 0,
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

    pub fn current_selection(&self) -> &SingleSelectorItem<V, L> {
        &self.options[self.selected_index]
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
                Span::styled(selected.label.to_string(), selected.label_style),
                Span::from(" >"),
            ])
            .style(style)
            .centered(),
        );
    }
}
