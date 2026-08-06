use std::future::Future;

use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::runtime::Handle;

use crate::error::AppError;
use crate::models::{Project, State, Task};
use crate::store::TaskStore;

use super::view;

pub struct App {
    pub project: Project,
    pub states: Vec<State>,
    pub tasks: Vec<Task>,
    pub selected_index: usize,

    pub scroll_offset: u16,
    pub input: InputState,
    pub popup: Option<PopupState>,
    pub running: bool,
    pub error_message: String,

    // maps visual task position (0..n) to index in self.tasks, respecting state grouping order
    visual_order: Vec<usize>,
}

pub struct InputState {
    pub buffer: String,
    pub cursor_position: usize,
    pub focused: bool,
}

pub enum PopupState {
    TaskDetail {
        task_id: i64,
    },
    StatePicker {
        task_id: i64,
        selected_state_index: usize,
    },
    ConfirmDelete {
        task_id: i64,
        task_title: String,
        confirm: bool,
    },
}

enum Message {
    // navigation
    MoveUp,
    MoveDown,
    FocusInput,
    Quit,

    // input bar
    TypeChar(char),
    DeleteChar,
    CursorLeft,
    CursorRight,
    SubmitInput(String),
    CancelInput,

    // task actions
    OpenDetail(i64),
    OpenMovePicker(i64),
    OpenDeleteConfirm(i64),

    // popup interactions
    DismissPopup,
    MovePickerUp,
    MovePickerDown,
    ConfirmMove(i64, String),
    ExecuteDelete(i64),
    ToggleDeleteConfirm,

    // side effects
    DataRefreshed(Vec<State>, Vec<Task>),
    ErrorOccurred(String),
}

impl App {
    pub fn new(project: Project, states: Vec<State>, tasks: Vec<Task>) -> Self {
        let mut app = App {
            project,
            states,
            tasks,
            selected_index: 0,
            scroll_offset: 0,
            input: InputState {
                buffer: String::new(),
                cursor_position: 0,
                focused: false,
            },
            popup: None,
            running: true,
            error_message: String::new(),

            visual_order: Vec::new(),
        };
        app.rebuild_visual_order();
        app
    }

    pub async fn from_store<S: TaskStore + Sync>(store: &S) -> Result<Self, AppError> {
        let project = Self::block_on(store.get_active_project())
            .map_err(|e| AppError::Internal(format!("failed to load active project: {}", e)))?;
        let (states, tasks) = Self::fetch_data(store, project.id)?;
        Ok(App::new(project, states, tasks))
    }

    pub async fn run<S: TaskStore + Sync>(&mut self, store: &S) -> Result<(), AppError> {
        enable_raw_mode()
            .map_err(|e| AppError::Internal(format!("failed to enable raw mode: {}", e)))?;
        let mut stdout = std::io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            SetCursorStyle::BlinkingBlock,
            Show
        )
        .map_err(|e| AppError::Internal(format!("failed to enter alternate screen: {}", e)))?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| AppError::Internal(format!("failed to create terminal: {}", e)))?;

        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            original_hook(info);
        }));

        let result = self.event_loop(&mut terminal, store).await;

        disable_raw_mode()
            .map_err(|e| AppError::Internal(format!("failed to disable raw mode: {}", e)))?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .map_err(|e| AppError::Internal(format!("failed to leave alternate screen: {}", e)))?;
        terminal
            .show_cursor()
            .map_err(|e| AppError::Internal(format!("failed to show cursor: {}", e)))?;

        result
    }

    pub fn is_input_selected(&self) -> bool {
        self.selected_index == self.visual_order.len()
    }

    pub fn selected_task(&self) -> Option<&Task> {
        if self.selected_index < self.visual_order.len() {
            Some(&self.tasks[self.visual_order[self.selected_index]])
        } else {
            None
        }
    }

    fn block_on<T>(f: impl Future<Output = T>) -> T {
        tokio::task::block_in_place(|| Handle::current().block_on(f))
    }

    fn selectable_row_count(&self) -> usize {
        self.visual_order.len() + 1
    }

    fn select_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn select_down(&mut self) {
        let max = self.selectable_row_count() - 1;
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    fn ensure_row_visible(&mut self, row_index: u16, viewport_height: u16) {
        if viewport_height == 0 {
            return;
        }
        if row_index < self.scroll_offset {
            self.scroll_offset = row_index;
        } else if row_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = row_index - viewport_height + 1;
        }
    }

    fn set_error(&mut self, msg: String) {
        self.error_message = msg;
    }

    fn rebuild_visual_order(&mut self) {
        self.visual_order.clear();
        for state in &self.states {
            for (task_idx, task) in self.tasks.iter().enumerate() {
                if task.state_id == state.id {
                    self.visual_order.push(task_idx);
                }
            }
        }
    }

    fn scroll_to_selection(&mut self) {
        let mut line_index: u16 = 0;

        if self.selected_index >= self.visual_order.len() {
            // input bar selected — scroll past all tasks
            for state in &self.states {
                line_index += 1; // header
                let count = self.tasks.iter().filter(|t| t.state_id == state.id).count();
                line_index += count as u16;
            }
        } else {
            let flat_idx = self.visual_order[self.selected_index];
            let selected_state_id = self.tasks[flat_idx].state_id;

            for state in &self.states {
                line_index += 1; // state header

                if state.id == selected_state_id {
                    // count tasks in this state before the selected one
                    let tasks_in_state: Vec<_> = self
                        .tasks
                        .iter()
                        .filter(|t| t.state_id == state.id)
                        .collect();
                    let pos = tasks_in_state
                        .iter()
                        .position(|t| t.id == self.tasks[flat_idx].id)
                        .unwrap_or(0);
                    line_index += pos as u16;
                    break;
                }

                let count = self.tasks.iter().filter(|t| t.state_id == state.id).count();
                line_index += count as u16;
            }
        }

        let viewport_height = 20u16;
        self.ensure_row_visible(line_index, viewport_height);
    }

    fn fetch_data<S: TaskStore + Sync>(
        store: &S,
        project_id: i64,
    ) -> Result<(Vec<State>, Vec<Task>), AppError> {
        let states = Self::block_on(store.list_states(project_id))
            .map_err(|e| AppError::Internal(format!("failed to list states: {}", e)))?;
        let tasks = Self::block_on(store.list_tasks(project_id, None))
            .map_err(|e| AppError::Internal(format!("failed to list tasks: {}", e)))?;
        Ok((states, tasks))
    }

    fn refresh_after_mutation<S: TaskStore + Sync>(&self, store: &S) -> Option<Message> {
        match Self::fetch_data(store, self.project.id) {
            Ok((states, tasks)) => Some(Message::DataRefreshed(states, tasks)),
            Err(e) => Some(Message::ErrorOccurred(format!("{}", e))),
        }
    }

    async fn event_loop<S: TaskStore + Sync>(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        store: &S,
    ) -> Result<(), AppError> {
        // initial render
        terminal
            .draw(|f| view::render(f, self))
            .map_err(|e| AppError::Internal(format!("render error: {}", e)))?;

        while self.running {
            // block until an event arrives — no wasted CPU
            match event::read().map_err(|e| AppError::Internal(format!("event error: {}", e)))? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // clear any stale error before processing new input
                    self.error_message.clear();

                    // map raw key to a semantic Message
                    if let Some(msg) = self.map_key_to_message(key) {
                        // cascade: one message can produce another
                        let mut next = Some(msg);
                        while let Some(m) = next {
                            next = self.update(store, m);
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // terminal was resized — just redraw
                }
                _ => {}
            }

            // redraw once after all cascaded updates
            terminal
                .draw(|f| view::render(f, self))
                .map_err(|e| AppError::Internal(format!("render error: {}", e)))?;
        }

        Ok(())
    }

    fn map_key_to_message(&self, key: KeyEvent) -> Option<Message> {
        // popup mode — only popup keys are valid
        if self.popup.is_some() {
            return self.popup_key_to_message(key.code);
        }

        // input mode — typing keys go to the input handler
        if self.input.focused {
            return self.input_key_to_message(key.code);
        }

        // normal navigation mode
        self.navigation_key_to_message(key.code)
    }

    fn navigation_key_to_message(&self, code: KeyCode) -> Option<Message> {
        match code {
            KeyCode::Up => Some(Message::MoveUp),
            KeyCode::Down => Some(Message::MoveDown),
            KeyCode::Enter => {
                if self.is_input_selected() {
                    Some(Message::FocusInput)
                } else {
                    self.selected_task().map(|t| Message::OpenDetail(t.id))
                }
            }
            KeyCode::Char('a') => Some(Message::FocusInput),
            KeyCode::Char('m') => self.selected_task().map(|t| Message::OpenMovePicker(t.id)),
            KeyCode::Char('d') => self
                .selected_task()
                .map(|t| Message::OpenDeleteConfirm(t.id)),
            KeyCode::Char('q') => Some(Message::Quit),
            _ => None,
        }
    }

    fn input_key_to_message(&self, code: KeyCode) -> Option<Message> {
        match code {
            KeyCode::Esc => Some(Message::CancelInput),
            KeyCode::Enter => {
                let title = self.input.buffer.trim().to_string();
                if title.is_empty() {
                    Some(Message::CancelInput)
                } else {
                    Some(Message::SubmitInput(title))
                }
            }
            KeyCode::Char(c) => Some(Message::TypeChar(c)),
            KeyCode::Backspace => Some(Message::DeleteChar),
            KeyCode::Left => Some(Message::CursorLeft),
            KeyCode::Right => Some(Message::CursorRight),
            _ => None,
        }
    }

    fn popup_key_to_message(&self, code: KeyCode) -> Option<Message> {
        match self.popup.as_ref().unwrap() {
            PopupState::TaskDetail { .. } => match code {
                KeyCode::Esc | KeyCode::Enter => Some(Message::DismissPopup),
                _ => None,
            },
            PopupState::StatePicker { task_id, .. } => {
                let task_id = *task_id;
                match code {
                    KeyCode::Esc => Some(Message::DismissPopup),
                    KeyCode::Up => Some(Message::MovePickerUp),
                    KeyCode::Down => Some(Message::MovePickerDown),
                    KeyCode::Enter => {
                        let state_name = match self.popup.as_ref().unwrap() {
                            PopupState::StatePicker {
                                selected_state_index,
                                ..
                            } => self.states[*selected_state_index].name.clone(),
                            _ => unreachable!(),
                        };
                        Some(Message::ConfirmMove(task_id, state_name))
                    }
                    _ => None,
                }
            }
            PopupState::ConfirmDelete {
                task_id, confirm, ..
            } => {
                let task_id = *task_id;
                match code {
                    KeyCode::Esc | KeyCode::Char('n') => Some(Message::DismissPopup),
                    KeyCode::Char('y') => Some(Message::ExecuteDelete(task_id)),
                    KeyCode::Enter => {
                        if *confirm {
                            Some(Message::ExecuteDelete(task_id))
                        } else {
                            Some(Message::DismissPopup)
                        }
                    }
                    KeyCode::Left | KeyCode::Right => Some(Message::ToggleDeleteConfirm),
                    _ => None,
                }
            }
        }
    }

    // update takes a Message, mutates state, optionally returns a follow-up Message
    fn update<S: TaskStore + Sync>(&mut self, store: &S, msg: Message) -> Option<Message> {
        match msg {
            // navigation
            Message::MoveUp => {
                self.select_up();
                self.scroll_to_selection();
                None
            }
            Message::MoveDown => {
                self.select_down();
                self.scroll_to_selection();
                None
            }
            Message::FocusInput => {
                self.selected_index = self.visual_order.len();
                self.input.focused = true;
                self.input.buffer.clear();
                self.input.cursor_position = 0;
                self.scroll_to_selection();
                None
            }
            Message::Quit => {
                self.running = false;
                None
            }

            // input bar
            Message::TypeChar(c) => {
                self.input.buffer.insert(self.input.cursor_position, c);
                self.input.cursor_position += 1;
                None
            }
            Message::DeleteChar => {
                if self.input.cursor_position > 0 {
                    self.input.cursor_position -= 1;
                    self.input.buffer.remove(self.input.cursor_position);
                }
                None
            }
            Message::CursorLeft => {
                if self.input.cursor_position > 0 {
                    self.input.cursor_position -= 1;
                }
                None
            }
            Message::CursorRight => {
                if self.input.cursor_position < self.input.buffer.len() {
                    self.input.cursor_position += 1;
                }
                None
            }
            Message::SubmitInput(title) => {
                self.input.focused = false;
                self.input.buffer.clear();
                self.input.cursor_position = 0;

                match Self::block_on(store.add_task(&title, self.project.id)) {
                    Ok(_) => self.refresh_after_mutation(store),
                    Err(e) => Some(Message::ErrorOccurred(format!("{}", e))),
                }
            }
            Message::CancelInput => {
                self.input.focused = false;
                self.input.buffer.clear();
                self.input.cursor_position = 0;
                None
            }

            // task actions
            Message::OpenDetail(id) => {
                self.popup = Some(PopupState::TaskDetail { task_id: id });
                None
            }
            Message::OpenMovePicker(id) => {
                let idx = self
                    .selected_task()
                    .and_then(|t| self.states.iter().position(|s| s.id == t.state_id))
                    .unwrap_or(0);
                self.popup = Some(PopupState::StatePicker {
                    task_id: id,
                    selected_state_index: idx,
                });
                None
            }
            Message::OpenDeleteConfirm(id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == id) {
                    self.popup = Some(PopupState::ConfirmDelete {
                        task_id: id,
                        task_title: task.title.clone(),
                        confirm: false,
                    });
                }
                None
            }

            // popup interactions
            Message::DismissPopup => {
                self.popup = None;
                None
            }
            Message::MovePickerUp => {
                if let Some(PopupState::StatePicker {
                    selected_state_index,
                    ..
                }) = &mut self.popup
                    && *selected_state_index > 0
                {
                    *selected_state_index -= 1;
                }
                None
            }
            Message::MovePickerDown => {
                if let Some(PopupState::StatePicker {
                    selected_state_index,
                    ..
                }) = &mut self.popup
                    && *selected_state_index + 1 < self.states.len()
                {
                    *selected_state_index += 1;
                }
                None
            }
            Message::ConfirmMove(task_id, state_name) => {
                self.popup = None;
                match Self::block_on(store.move_task(task_id, self.project.id, &state_name)) {
                    Ok(_) => self.refresh_after_mutation(store),
                    Err(e) => Some(Message::ErrorOccurred(format!("{}", e))),
                }
            }
            Message::ExecuteDelete(task_id) => {
                self.popup = None;
                match Self::block_on(store.delete_task(task_id, self.project.id)) {
                    Ok(_) => self.refresh_after_mutation(store),
                    Err(e) => Some(Message::ErrorOccurred(format!("{}", e))),
                }
            }
            Message::ToggleDeleteConfirm => {
                if let Some(PopupState::ConfirmDelete { confirm, .. }) = &mut self.popup {
                    *confirm = !*confirm;
                }
                None
            }

            // side effects
            Message::DataRefreshed(states, tasks) => {
                self.states = states;
                self.tasks = tasks;
                self.rebuild_visual_order();
                if self.selected_index >= self.selectable_row_count() {
                    self.selected_index = self.selectable_row_count().saturating_sub(1);
                }
                self.scroll_to_selection();
                None
            }
            Message::ErrorOccurred(msg) => {
                self.set_error(msg);
                None
            }
        }
    }
}
