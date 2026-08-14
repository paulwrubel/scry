use crate::models::{Task, TaskID};
use crate::tui::action::Action;
use crate::tui::component::popup::{ConfirmDelete, StatusSelection, TaskDetail};
use crate::tui::component::{HintBar, InputBar, ProjectStatusTasks, State, TaskList};
use crate::tui::component::{Popup, RenderContext};
use crossterm::event::KeyModifiers;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders};

#[derive(Debug, Clone, Copy)]
enum SelectedTask {
    First,
    ID(TaskID),
}

pub struct Root {
    task_list: TaskList,
    input_bar: InputBar,
    hint_bar: HintBar,
    popup: Option<Popup>,

    selected_task: Option<SelectedTask>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            task_list: TaskList::new(true),
            input_bar: InputBar::new(false),
            hint_bar: HintBar::new(),
            popup: None,

            selected_task: Some(SelectedTask::First),
        }
    }

    fn handle_action(&mut self, state: &State, action: Action) -> Option<Action> {
        match action {
            // UI actions — handled here, never reach the coordinator
            Action::FocusInput => {
                self.input_bar.focus();
                self.task_list.is_focused = false;
                None
            }
            Action::OpenPopupTaskDetail(task_id) => {
                self.popup = Some(Popup::TaskDetail(TaskDetail::new(task_id)));
                None
            }
            Action::OpenPopupMovePicker(task_id) => {
                let current_status_id = state
                    .tasks
                    .iter()
                    .find(|t| t.id == task_id)
                    .map(|t| t.status_id);
                self.popup = Some(Popup::StatusSelection(StatusSelection::new(
                    task_id,
                    state.statuses.len(),
                    current_status_id,
                )));
                None
            }
            Action::OpenPopupDeleteConfirm(task_id) => {
                if let Some(task) = state.tasks.iter().find(|t| t.id == task_id) {
                    self.popup = Some(Popup::ConfirmDelete(ConfirmDelete::new(
                        task_id,
                        task.title.clone(),
                    )));
                }
                None
            }
            Action::DismissPopup => {
                self.popup = None;
                None
            }
            Action::MoveFocusDown => {
                self.input_bar.focus();
                None
            }
            Action::MoveFocusUp => None,

            // Store actions that also dismiss the popup before bubbling
            Action::MoveTask { .. } => {
                self.popup = None;
                Some(action)
            }
            Action::DeleteTask(task_id) => {
                self.popup = None;

                let project_status_tasks: ProjectStatusTasks = state.into();

                // these really should match, i don't see how they couldn't, but still, we'll make sure
                if let Some(st) = self.task_from_selected_task(&project_status_tasks)
                    && st.id == task_id
                {
                    if let Some(next) = project_status_tasks.next(task_id) {
                        self.selected_task = Some(SelectedTask::ID(next.id))
                    } else if let Some(previous) = project_status_tasks.previous(task_id) {
                        self.selected_task = Some(SelectedTask::ID(previous.id))
                    } else {
                        self.selected_task = Some(SelectedTask::First)
                    }
                }

                Some(action)
            }

            // unhandled actions bubble up unchanged
            _ => Some(action),
        }
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // active popup swallows all input
        if let Some(ref mut popup) = self.popup {
            return popup
                .handle_event(state, key)
                .and_then(|a| self.handle_action(state, a));
        }

        // input bar when typing
        if self.input_bar.is_focused
            && let Some(action) = self.input_bar.handle_event(state, key)
        {
            return match action {
                Action::MoveFocusDown => None,
                Action::MoveFocusUp => {
                    self.input_bar.blur();
                    self.task_list.is_focused = true;
                    None
                }
                _ => self.handle_action(state, action),
            };
        }

        // global keys are handled ONLY if nothing above handled the event
        match (key.modifiers, code) {
            (_, KeyCode::Enter | KeyCode::Char('m') | KeyCode::Char('d'))
                if self.task_list.is_focused =>
            {
                let project_status_tasks: ProjectStatusTasks = state.into();
                let selected = self.task_from_selected_task(&project_status_tasks);
                match (code, selected) {
                    (KeyCode::Enter, Some(task)) => {
                        self.handle_action(state, Action::OpenPopupTaskDetail(task.id))
                    }
                    (KeyCode::Char('m'), Some(task)) => {
                        self.handle_action(state, Action::OpenPopupMovePicker(task.id))
                    }
                    (KeyCode::Char('d'), Some(task)) => {
                        self.handle_action(state, Action::OpenPopupDeleteConfirm(task.id))
                    }
                    _ => None,
                }
            }
            (_, KeyCode::Up | KeyCode::Char('k')) if self.task_list.is_focused => {
                let project_status_tasks: ProjectStatusTasks = state.into();
                // check if we have a current selection AND if there's a "previous task"
                if let Some(next_task) = self
                    .task_from_selected_task(&project_status_tasks)
                    .and_then(|task| project_status_tasks.previous(task.id))
                {
                    self.selected_task = Some(SelectedTask::ID(next_task.id));
                }

                None
            }
            (_, KeyCode::Down | KeyCode::Char('j')) if self.task_list.is_focused => {
                // if the task list is focused, we may have to change the selected task
                let project_status_tasks: ProjectStatusTasks = state.into();
                // check if we have a current selection AND if there's a "next task"
                if let Some(next_task) = self
                    .task_from_selected_task(&project_status_tasks)
                    .and_then(|task| project_status_tasks.next(task.id))
                {
                    self.selected_task = Some(SelectedTask::ID(next_task.id));
                } else {
                    self.task_list.is_focused = false;
                    self.input_bar.focus();
                }
                None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.handle_action(state, Action::Quit),
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.handle_action(state, Action::FocusInput)
            }
            _ => None,
        }
    }

    pub fn render(&mut self, ctx: &mut RenderContext) {
        // create the full bordered area
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" scry ")
            .title(format!(" {} ", ctx.state.project.name));
        // render it to the entire window
        let content_area = block.inner(ctx.area);

        // add left padding
        let left_padding = 1;
        let [_, content_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_padding), // left padding
                Constraint::Min(0),               // content
            ])
            .areas(content_area);

        let top_padding = 1;
        let [_, task_list_area, input_bar_area, hints_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_padding), // top padding
                Constraint::Min(0),              // task list
                Constraint::Length(3),           // input bar
                Constraint::Length(1),           // hint bar
            ])
            .areas(content_area);

        // render components in z-order

        let project_status_tasks: ProjectStatusTasks = ctx.state.into();
        let selected_task = self.task_from_selected_task(&project_status_tasks);
        self.task_list.render(
            &mut RenderContext {
                state: ctx.state,
                frame: ctx.frame,
                area: task_list_area,
            },
            &project_status_tasks,
            selected_task.map(|t| t.id),
        );
        self.input_bar.render(&mut RenderContext {
            state: ctx.state,
            frame: ctx.frame,
            area: input_bar_area,
        });
        self.hint_bar.render(&mut RenderContext {
            state: ctx.state,
            frame: ctx.frame,
            area: hints_area,
        });
        ctx.render_widget(block);

        // popup last (on top of everything)
        if let Some(ref popup) = self.popup {
            popup.render(ctx);
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.hint_bar.set_message(msg);
    }

    pub fn clear_status(&mut self) {
        self.hint_bar.set_message(String::new());
    }

    fn task_from_selected_task<'a>(
        &self,
        project_status_tasks: &'a ProjectStatusTasks,
    ) -> Option<&'a Task> {
        match self.selected_task {
            Some(st) => match st {
                SelectedTask::First => project_status_tasks.first(),
                SelectedTask::ID(task_id) => project_status_tasks.get_task_by_id(task_id),
            },
            None => None,
        }
    }
}
