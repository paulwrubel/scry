use crossterm::event::KeyModifiers;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders};

use crate::tui::action::Action;
use crate::tui::component::popup::{ConfirmDelete, StatusSelection, TaskDetail};
use crate::tui::component::{HintBar, InputBar, State, TaskList};
use crate::tui::component::{Popup, RenderContext};

pub struct Root {
    task_list: TaskList,
    input_bar: InputBar,
    hint_bar: HintBar,

    popup: Option<Popup>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            task_list: TaskList::new(true),
            input_bar: InputBar::new(false),
            hint_bar: HintBar::new(),
            popup: None,
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.hint_bar.set_message(msg);
    }

    pub fn clear_status(&mut self) {
        self.hint_bar.set_message(String::new());
    }

    fn process(&mut self, state: &State, action: Action) -> Option<Action> {
        match action {
            // UI actions — handled here, never reach the coordinator
            Action::FocusInput => {
                self.input_bar.focus();
                self.task_list.blur();
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
            Action::MoveTask { .. } | Action::DeleteTask(_) => {
                self.popup = None;
                Some(action)
            }

            // unhandled actions bubble up unchanged
            _ => Some(action),
        }
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // 1 - active popup swallows all input
        if let Some(ref mut popup) = self.popup {
            return popup
                .handle_event(state, key)
                .and_then(|a| self.process(state, a));
        }

        // 2 - input bar when typing
        if self.input_bar.is_focused
            && let Some(action) = self.input_bar.handle_event(state, key)
        {
            return match action {
                Action::MoveFocusDown => None,
                Action::MoveFocusUp => {
                    self.input_bar.blur();
                    self.task_list.focus_index(state.tasks.len() - 1);
                    None
                }
                _ => self.process(state, action),
            };
        }

        // 3 - task list catched the next event if focused
        if self.task_list.is_focused
            && let Some(action) = self.task_list.handle_event(state, key)
        {
            return match action {
                Action::MoveFocusDown => {
                    self.task_list.blur();
                    self.input_bar.focus();
                    return None;
                }
                Action::MoveFocusUp => None,
                _ => self.process(state, action),
            };
        }

        // 4 - global keys are handled ONLY if nothing above handled the event
        match (key.modifiers, code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.process(state, Action::Quit),
            (KeyModifiers::NONE, KeyCode::Char('a')) => self.process(state, Action::FocusInput),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" scry ")
            .title(format!(" {} ", ctx.state.project.name));
        let inner_area = block.inner(ctx.area);

        let h_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(h_layout[1]);

        // render components in z-order

        self.task_list.render(&mut RenderContext {
            state: ctx.state,
            frame: ctx.frame,
            area: layout[1],
        });
        self.input_bar.render(&mut RenderContext {
            state: ctx.state,
            frame: ctx.frame,
            area: layout[2],
        });
        self.hint_bar.render(&mut RenderContext {
            state: ctx.state,
            frame: ctx.frame,
            area: layout[3],
        });

        // border on top at edges
        ctx.render_widget(block);

        // popup last (on top of everything)
        if let Some(ref popup) = self.popup {
            popup.render(ctx);
        }
    }
}
