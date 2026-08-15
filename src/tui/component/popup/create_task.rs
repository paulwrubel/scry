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
            title_input: InputBlock::new(
                String::from("Title"),
                true,
                Some(String::from("Do the laundry")),
            ),
            create_task_button: Button::new(String::from("Create Task"), false),
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
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Some(Block::default().borders(Borders::ALL).title("Create Task")),
        );

        let [title_input_area, _, create_button_area, _] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(0),
        ])
        .areas(content_area);

        self.title_input.render(&mut RenderContext {
            frame: ctx.frame,
            state: ctx.state,
            area: title_input_area,
        });

        self.create_task_button.render(&mut RenderContext {
            frame: ctx.frame,
            state: ctx.state,
            area: create_button_area,
        });

        // ctx.frame.render_widget(
        //     Paragraph::new(all_lines).wrap(Wrap { trim: false }),
        //     content_area,
        // );
    }
}
