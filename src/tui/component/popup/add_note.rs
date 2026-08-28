use crate::models::{Note, TaskID};
use crate::tui::action::Action;
use crate::tui::component::{Button, InputBlock};
use crate::tui::component::{ProjectState, RenderContext};
use chrono::DateTime;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Borders};

pub struct AddNote {
    contents_input: InputBlock,
    confirm_button: Button,

    task_id: TaskID,
}

impl AddNote {
    pub fn new(task_id: TaskID) -> Self {
        Self {
            contents_input: InputBlock::new(true, true)
                .with_title(String::from("Note"))
                .with_placeholder_text(String::from(
                    r#"Made some progress yesterday...

... but there's still a lot to do!
"#,
                )),
            confirm_button: Button::new(false, String::from("Add Note")),

            task_id,
        }
    }

    pub fn handle_event(&mut self, state: &ProjectState, key: KeyEvent) -> Option<Action> {
        // capture the content input's edit mode before forwarding, so the
        // match below can avoid fighting it for Esc/Up/Down
        let editing = self.contents_input.is_focused && self.contents_input.is_editing();

        if self.contents_input.is_focused {
            self.contents_input.handle_event(state, key);
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.handle_create_or_update(state),
            (_, KeyCode::Enter) => {
                if self.confirm_button.is_focused {
                    self.handle_create_or_update(state)
                } else {
                    None
                }
            }
            (_, KeyCode::Esc) => {
                if editing {
                    // the multiline input already exited edit mode
                    None
                } else {
                    Some(Action::DismissPopup)
                }
            }
            (_, KeyCode::Up) => {
                if editing {
                    // the multiline input moved the cursor up
                    None
                } else if self.confirm_button.is_focused {
                    self.confirm_button.blur();
                    self.contents_input.focus();
                    None
                } else {
                    None
                }
            }
            (_, KeyCode::Down) => {
                if editing {
                    // the multiline input moved the cursor down
                    None
                } else if self.contents_input.is_focused {
                    self.contents_input.blur();
                    self.confirm_button.focus();
                    None
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(50),
            Constraint::Percentage(60),
            Some(Block::default().borders(Borders::ALL).title("Add Note")),
        );

        let [description_input_area, _, create_button_area] =
            content_area.layout(&Layout::vertical([
                Constraint::Fill(1),   // most of the space should be the input
                Constraint::Length(1), // padding
                Constraint::Length(1),
            ]));

        self.contents_input
            .render(&mut ctx.with_area(description_input_area));

        self.confirm_button
            .render(&mut ctx.with_area(create_button_area));
    }

    fn handle_create_or_update(&self, _state: &ProjectState) -> Option<Action> {
        if self.is_valid() {
            let contents = self.contents_input.buffer_text();

            Some(Action::CreateNote(Note {
                id: 0, // dummy id
                task_id: self.task_id,
                contents,
                created_at: DateTime::default(), // dummy, overwritten on insert
            }))
        } else {
            None
        }
    }

    fn is_valid(&self) -> bool {
        !self.contents_input.buffer_text().is_empty()
    }
}
