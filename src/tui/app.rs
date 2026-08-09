use std::future::Future;

use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::runtime::Handle;

use crate::error::AppError;
use crate::models::{Project, State, Task};
use crate::store::TaskStore;

use crate::tui::action::Action;
use crate::tui::component::AppContext;
use crate::tui::component::Root;

/// Terminal lifecycle and domain logic. UI orchestration lives in Root.
pub struct App {
    root: Root,
    is_running: bool,

    // domain state
    project: Project,
    states: Vec<State>,
    tasks: Vec<Task>,
}

impl App {
    pub fn new(project: Project, states: Vec<State>, tasks: Vec<Task>) -> Self {
        App {
            root: Root::new(),
            is_running: true,
            project,
            states,
            tasks,
        }
    }

    pub async fn from_store<S: TaskStore + Sync>(store: &S) -> Result<Self, AppError> {
        let project = Self::block_on(store.get_active_project())
            .map_err(|e| AppError::Internal(format!("failed to load active project: {}", e)))?;
        let (states, tasks) = Self::fetch_data(store, project.id)?;
        Ok(App::new(project, states, tasks))
    }

    pub async fn run<S: TaskStore + Sync>(&mut self, store: &S) -> Result<(), AppError> {
        let mut terminal = Self::setup_terminal()?;

        let result = self.event_loop(&mut terminal, store).await;

        Self::teardown_terminal(terminal)?;

        result
    }

    fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, AppError> {
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

        let terminal = Terminal::new(CrosstermBackend::new(stdout))
            .map_err(|e| AppError::Internal(format!("failed to create terminal: {}", e)))?;

        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            original_hook(info);
        }));

        Ok(terminal)
    }

    fn teardown_terminal(
        mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), AppError> {
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

        Ok(())
    }

    async fn event_loop<S: TaskStore + Sync>(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        store: &S,
    ) -> Result<(), AppError> {
        while self.is_running {
            let ctx = AppContext {
                project: &self.project,
                states: &self.states,
                tasks: &self.tasks,
            };

            terminal
                .draw(|f| self.root.render(&ctx, f))
                .map_err(|e| AppError::Internal(format!("render error: {}", e)))?;

            match event::read().map_err(|e| AppError::Internal(format!("event error: {}", e)))? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    self.root.clear_status();
                    if let Some(action) = self.root.handle_event(&ctx, key) {
                        self.process_action(store, action);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn process_action<S: TaskStore + Sync>(&mut self, store: &S, action: Action) {
        match action {
            Action::Quit => self.is_running = false,

            Action::AddTask(title) => {
                match Self::block_on(store.add_task(&title, self.project.id)) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::MoveTask {
                task_id,
                state_name,
            } => match Self::block_on(store.move_task(task_id, self.project.id, &state_name)) {
                Ok(_) => self.refresh_data(store),
                Err(e) => self.root.set_status(format!("{}", e)),
            },
            Action::DeleteTask(task_id) => {
                match Self::block_on(store.delete_task(task_id, self.project.id)) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::RenameProject(new_name) => {
                match Self::block_on(store.rename_project(self.project.id, &new_name)) {
                    Ok(_) => {
                        self.project.name = new_name;
                        self.refresh_data(store);
                    }
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::RenameState { old_name, new_name } => {
                match Self::block_on(store.rename_state(self.project.id, &old_name, &new_name)) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::AddState(name) => {
                match Self::block_on(store.add_state(self.project.id, &name)) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::DeleteState(state_name) => {
                match Self::block_on(store.remove_state(self.project.id, &state_name, false)) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::SetStateColor { state_id, color } => {
                let state_name = self
                    .states
                    .iter()
                    .find(|s| s.id == state_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("");
                match Self::block_on(store.set_state_color(
                    self.project.id,
                    state_name,
                    color.as_deref(),
                )) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            Action::ReorderState {
                state_name,
                new_position,
            } => {
                match Self::block_on(store.reorder_state(
                    self.project.id,
                    &state_name,
                    new_position,
                )) {
                    Ok(_) => self.refresh_data(store),
                    Err(e) => self.root.set_status(format!("{}", e)),
                }
            }
            _ => {}
        }
    }

    fn block_on<T>(f: impl Future<Output = T>) -> T {
        tokio::task::block_in_place(|| Handle::current().block_on(f))
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

    fn refresh_data<S: TaskStore + Sync>(&mut self, store: &S) {
        match Self::fetch_data(store, self.project.id) {
            Ok((states, tasks)) => {
                self.states = states;
                self.tasks = tasks;
            }
            Err(e) => self.root.set_status(format!("{}", e)),
        }
    }
}
