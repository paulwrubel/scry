use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};

use crate::tui::action::Action;
use crate::tui::component::AppContext;
use crate::tui::component::Component;
use crate::tui::component::Popup;
use crate::tui::component::popup::{ConfirmDelete, ProjectSettings, StatePicker, TaskDetail};
use crate::tui::component::{InputBar, StatusBar, TaskList};

pub struct Root {
    // ── children ──
    task_list: TaskList,
    input_bar: InputBar,
    status_bar: StatusBar,
    popup: Option<Popup>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            task_list: TaskList::new(),
            input_bar: InputBar::new(),
            status_bar: StatusBar::new(),
            popup: None,
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_bar.set_message(msg);
    }

    pub fn clear_status(&mut self) {
        self.status_bar.set_message(String::new());
    }

    fn process(&mut self, ctx: &AppContext, action: Action) -> Option<Action> {
        match action {
            // UI actions — handled here, never reach the coordinator
            Action::FocusInput => {
                self.input_bar.focus();
                None
            }
            Action::OpenPopupTaskDetail(task_id) => {
                self.popup = Some(Popup::TaskDetail(TaskDetail::new(task_id)));
                None
            }
            Action::OpenPopupMovePicker(task_id) => {
                let current_state_id = ctx
                    .tasks
                    .iter()
                    .find(|t| t.id == task_id)
                    .map(|t| t.state_id);
                self.popup = Some(Popup::StatePicker(StatePicker::new(
                    task_id,
                    ctx.states.len(),
                    current_state_id,
                )));
                None
            }
            Action::OpenPopupDeleteConfirm(task_id) => {
                if let Some(task) = ctx.tasks.iter().find(|t| t.id == task_id) {
                    self.popup = Some(Popup::ConfirmDelete(ConfirmDelete::new(
                        task_id,
                        task.title.clone(),
                    )));
                }
                None
            }
            Action::OpenPopupProjectSettings => {
                self.popup = Some(Popup::ProjectSettings(ProjectSettings::new(
                    ctx.states.len(),
                )));
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

            // Store actions — bubble up unchanged
            Action::Quit
            | Action::AddTask(_)
            | Action::RenameProject(_)
            | Action::RenameState { .. }
            | Action::AddState(_)
            | Action::DeleteState(_)
            | Action::SetStateColor { .. }
            | Action::ReorderState { .. } => Some(action),
        }
    }
}

impl Component for Root {
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // 1 - active popup swallows all input
        if let Some(ref mut popup) = self.popup {
            match popup {
                Popup::StatePicker(p) => p.sync(ctx),
                Popup::ProjectSettings(p) => p.sync(ctx),
                _ => {}
            }
            return popup
                .handle_event(ctx, key)
                .and_then(|a| self.process(ctx, a));
        }

        // 2 - input bar when typing
        if self.input_bar.is_focused() {
            return self
                .input_bar
                .handle_event(ctx, key)
                .and_then(|a| self.process(ctx, a));
        }

        // 3 - task list handles navigation + Enter/m/d internally
        if let Some(action) = self.task_list.handle_event(ctx, key) {
            return self.process(ctx, action);
        }

        // 4 - global keys are handled ONLY if nothing above handled the event
        match code {
            KeyCode::Char('q') => self.process(ctx, Action::Quit),
            KeyCode::Char('a') => self.process(ctx, Action::FocusInput),
            KeyCode::Char('s') => self.process(ctx, Action::OpenPopupProjectSettings),
            _ => None,
        }
    }

    fn render(&self, ctx: &AppContext, frame: &mut Frame, _area: Rect) {
        let area = frame.area();

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" scry ")
            .title(format!(" {} ", ctx.project.name));
        let inner_area = block.inner(area);

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
        self.task_list.render(ctx, frame, layout[1]);
        self.input_bar.render(ctx, frame, layout[2]);
        self.status_bar.render(ctx, frame, layout[3]);

        // border on top at edges
        frame.render_widget(block, area);

        // popup last (on top of everything)
        if let Some(ref popup) = self.popup {
            popup.render(ctx, frame, area);
        }
    }
}
