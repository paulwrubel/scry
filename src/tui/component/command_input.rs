use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    style::Style,
};

use crate::tui::{
    Action,
    component::{RenderContext, State, shared::Input},
};

pub struct CommandInput {
    pub is_focused: bool,

    input: Input,
}

impl CommandInput {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,

            input: Input::new(is_focused)
                .with_prefix_text(String::from("/"), Style::default().dim()),
        }
    }

    pub fn reset(&mut self) {
        self.input =
            Input::new(self.is_focused).with_prefix_text(String::from("/"), Style::default().dim())
    }

    pub fn focus(&mut self) {
        self.input.focus();
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.input.blur();
        self.is_focused = false;
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Vec<Action> {
        // input never sends an Action
        self.input.handle_event(state, key);

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => vec![Action::CloseCommandInput],
            (_, KeyCode::Enter) => {
                let mut actions = vec![Action::CloseCommandInput];

                let action = self.process_command_text();
                if let Some(action) = action {
                    self.reset();
                    actions.insert(0, action);
                }

                actions
            }
            _ => vec![],
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        self.input.render(ctx);
    }

    fn process_command_text(&self) -> Option<Action> {
        let text = self.input.buffer_text();

        None
    }
}
