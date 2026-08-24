use crate::tui::action::Action;
use crate::tui::component::{Button, InputBlock};
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Borders};

pub struct CreateTask {
    // task_id: i64,
    // task_title: String,
    // is_confirmation_option_highlighted: bool,
    title_input: InputBlock,
    create_task_button: Button,
}

impl CreateTask {
    pub fn new() -> Self {
        Self {
            title_input: InputBlock::new(true)
                .with_title(String::from("Title"))
                .with_placeholder_text(String::from("Do the laundry")),

            create_task_button: Button::new(false, String::from("Create Task")),
        }
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        if self.title_input.is_focused {
            // InputBlock always returns None, so no need to propagate
            self.title_input.handle_event(state, key);
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                if !self.title_input.buffer_text().is_empty() {
                    Some(Action::AddTask(self.title_input.buffer_text().to_string()))
                } else {
                    None
                }
            }
            (_, KeyCode::Enter) => {
                if self.create_task_button.is_focused && !self.title_input.buffer_text().is_empty()
                {
                    Some(Action::AddTask(self.title_input.buffer_text().to_string()))
                } else {
                    None
                }
            }
            (_, KeyCode::Esc) => Some(Action::DismissPopup),
            (_, KeyCode::Up) => {
                if self.create_task_button.is_focused {
                    self.create_task_button.blur();
                    self.title_input.focus();
                }
                None
            }
            (_, KeyCode::Down) => {
                if self.title_input.is_focused {
                    self.title_input.blur();
                    self.create_task_button.focus();
                }
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(50),
            Constraint::Percentage(60),
            Some(Block::default().borders(Borders::ALL).title("Create Task")),
        );

        let [title_input_area, _, create_button_area] = content_area.layout(&Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1), // remaining space
            Constraint::Length(1),
        ]));

        self.title_input
            .render(&mut ctx.with_area(title_input_area));

        self.create_task_button
            .render(&mut ctx.with_area(create_button_area));
    }
}
