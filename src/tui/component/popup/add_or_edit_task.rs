use crate::models::Task;
use crate::tui::action::Action;
use crate::tui::component::{Button, InputBlock, ProjectStatusTasks};
use crate::tui::component::{RenderContext, State};
use chrono::DateTime;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Borders};

pub struct AddOrEditTask {
    title_input: InputBlock,
    create_task_button: Button,

    task: Option<Task>,
}

impl AddOrEditTask {
    pub fn new(task: Option<Task>) -> Self {
        Self {
            title_input: InputBlock::new(true)
                .with_title(String::from("Title"))
                .with_placeholder_text(String::from("Do the laundry"))
                .with_text(task.as_ref().map(|t| t.title.clone()).unwrap_or_default()),
            create_task_button: Button::new(
                false,
                String::from(if task.is_none() {
                    "Create Task"
                } else {
                    "Edit Task"
                }),
            ),

            task,
        }
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        if self.title_input.is_focused {
            // InputBlock always returns None, so no need to propagate
            self.title_input.handle_event(state, key);
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.handle_create_or_update(state),
            (_, KeyCode::Enter) => {
                if self.create_task_button.is_focused {
                    self.handle_create_or_update(state)
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
            Some(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if self.task.is_none() {
                        "Create Task"
                    } else {
                        "Edit Task"
                    }),
            ),
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

    fn handle_create_or_update(&self, state: &State) -> Option<Action> {
        if self.is_valid() {
            match &self.task {
                Some(task) => Some(Action::UpdateTask(Task {
                    title: self.title_input.buffer_text().to_string(),
                    ..task.clone()
                })),
                None => {
                    let project_status_tasks = ProjectStatusTasks::from(state);

                    let Some(first_status) = state.statuses.first() else {
                        // no status to put a task in!
                        return None;
                    };

                    let last_position = project_status_tasks
                        .tasks_in_status(first_status.id)
                        .iter()
                        .map(|t| t.position)
                        .max();

                    Some(Action::CreateTask(Task {
                        id: 0, // dummy id
                        project_id: state.project.id,
                        title: self.title_input.buffer_text().to_string(),
                        description: None,
                        status_id: first_status.id,
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
