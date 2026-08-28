use crate::models::Task;
use crate::tui::action::Action;
use crate::tui::component::{Button, InputBlock};
use crate::tui::component::{ProjectState, RenderContext};
use crate::tui::state::TaskWithNotes;
use chrono::DateTime;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Borders};

pub struct AddOrEditTask {
    title_input: InputBlock,
    description_input: InputBlock,
    confirm_button: Button,

    task: Option<TaskWithNotes>,
}

impl AddOrEditTask {
    pub fn new(task: Option<TaskWithNotes>) -> Self {
        Self {
            title_input: InputBlock::new(true, false)
                .with_title(String::from("Title"))
                .with_placeholder_text(String::from("Do the laundry"))
                .with_text(
                    task.as_ref()
                        .map(|task| task.title.clone())
                        .unwrap_or_default(),
                ),
            description_input: InputBlock::new(false, true)
                .with_title(String::from("Description"))
                .with_placeholder_text(String::from(
                    r#"There's a lot of laundry to do...

1. Delicates
2. Towels
3. Whites
"#,
                ))
                .with_text(
                    task.as_ref()
                        .map(|t| t.description.clone().unwrap_or_default())
                        .unwrap_or_default(),
                ),
            confirm_button: Button::new(
                false,
                String::from(if task.is_none() {
                    "Add Task"
                } else {
                    "Edit Task"
                }),
            ),

            task,
        }
    }

    pub fn handle_event(&mut self, state: &ProjectState, key: KeyEvent) -> Option<Action> {
        // capture the description input's edit mode before forwarding, so the
        // match below can avoid fighting it for Esc/Up/Down
        let description_editing =
            self.description_input.is_focused && self.description_input.is_editing();

        if self.title_input.is_focused {
            // InputBlock always returns None, so no need to propagate
            self.title_input.handle_event(state, key);
        } else if self.description_input.is_focused {
            self.description_input.handle_event(state, key);
        }

        match (key.modifiers, key.code) {
            #[cfg(target_os = "macos")]
            (KeyModifiers::SUPER, KeyCode::Char('s')) => self.handle_create_or_update(state),
            #[cfg(not(target_os = "macos"))]
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.handle_create_or_update(state),
            (_, KeyCode::Enter) => {
                if self.confirm_button.is_focused {
                    self.handle_create_or_update(state)
                } else {
                    None
                }
            }
            (_, KeyCode::Esc) => {
                if description_editing {
                    // the multiline input already exited edit mode
                    None
                } else {
                    Some(Action::DismissPopup)
                }
            }
            (_, KeyCode::Up) => {
                if description_editing {
                    // the multiline input moved the cursor up
                    None
                } else if self.description_input.is_focused {
                    self.description_input.blur();
                    self.title_input.focus();
                    None
                } else if self.confirm_button.is_focused {
                    self.confirm_button.blur();
                    self.description_input.focus();
                    None
                } else {
                    None
                }
            }
            (_, KeyCode::Down) => {
                if description_editing {
                    // the multiline input moved the cursor down
                    None
                } else if self.title_input.is_focused {
                    self.title_input.blur();
                    self.description_input.focus();
                    None
                } else if self.description_input.is_focused {
                    self.description_input.blur();
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
            Some(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if self.task.is_none() {
                        "Add Task"
                    } else {
                        "Edit Task"
                    }),
            ),
        );

        let [
            title_input_area,
            description_input_area,
            _,
            confirm_button_area,
        ] = content_area.layout(&Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Fill(1), // remaining space
            Constraint::Length(1),
        ]));

        self.title_input
            .render(&mut ctx.with_area(title_input_area));

        self.description_input
            .render(&mut ctx.with_area(description_input_area));

        self.confirm_button
            .render(&mut ctx.with_area(confirm_button_area));
    }

    fn handle_create_or_update(&self, state: &ProjectState) -> Option<Action> {
        if self.is_valid() {
            let title = self.title_input.buffer_text();
            let description = {
                let text = self.description_input.buffer_text();
                if text.is_empty() { None } else { Some(text) }
            };
            match &self.task {
                Some(task) => Some(Action::UpdateTask(Task {
                    title,
                    description,
                    ..Task::from(task)
                })),
                None => {
                    let status = state
                        .project()
                        .entry_status_id
                        .and_then(|id| state.get_status_by_id(id))
                        .or_else(|| state.statuses().next())?;

                    let last_position = state
                        .tasks_in_status(status.id)
                        .iter()
                        .map(|t| t.position)
                        .max();

                    Some(Action::CreateTask(Task {
                        id: 0, // dummy id
                        project_id: state.project().id,
                        title,
                        description,
                        status_id: status.id,
                        position: last_position.map_or(0, |p| p + 1),
                        created_at: DateTime::default(), // dummy, overwritten on insert
                    }))
                }
            }
        } else {
            None
        }
    }

    fn is_valid(&self) -> bool {
        !self.title_input.buffer_text().is_empty()
    }
}
