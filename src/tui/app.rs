use crate::models::ProjectID;
use crate::store::TaskStore;
use crate::tui::action::Action;
use crate::tui::component::RenderContext;
use crate::tui::component::Root;
use crate::{error::AppError, tui::component::State};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::future::Future;
use tokio::runtime::Handle;

/// Terminal lifecycle and domain logic. UI orchestration lives in Root.
pub struct App<S: TaskStore + Sync> {
    root: Root,
    is_running: bool,

    // domain state
    store: S,
    project_id: ProjectID,
}

impl<S: TaskStore + Sync> App<S> {
    pub fn new(store: S, project_id: ProjectID) -> Self {
        App {
            root: Root::new(),
            is_running: true,
            store,
            project_id,
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        let mut terminal = Self::setup_terminal()?;

        let result = self.event_loop(&mut terminal).await;

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

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), AppError> {
        while self.is_running {
            let state = State::load_from_store(&self.store, self.project_id).await?;

            terminal
                .draw(|f| {
                    let area = f.area();
                    self.root.render(&mut RenderContext {
                        state: &state,
                        frame: f,
                        area,
                    })
                })
                .map_err(|e| AppError::Internal(format!("render error: {}", e)))?;

            match event::read().map_err(|e| AppError::Internal(format!("event error: {}", e)))? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    self.root.clear_status();
                    for action in self.root.handle_event(&state, key) {
                        self.process_action(action);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn process_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.is_running = false,

            Action::AddTask(title) => {
                match Self::block_on(self.store.add_task(&title, self.project_id)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::MoveTask { task_id, status_id } => {
                match Self::block_on(self.store.move_task(task_id, self.project_id, status_id)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::DeleteTask(task_id) => {
                match Self::block_on(self.store.delete_task(task_id, self.project_id)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::RenameProject(new_name) => {
                match Self::block_on(self.store.rename_project(self.project_id, &new_name)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::RenameStatus { id, new_name } => {
                match Self::block_on(self.store.rename_status(self.project_id, id, &new_name)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::AddStatus(name) => {
                match Self::block_on(self.store.add_status(self.project_id, &name)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::DeleteStatus(id) => {
                match Self::block_on(self.store.delete_status(self.project_id, id)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::SetStatusColor { status_id, color } => {
                let color_str = color.as_ref().map(|c| c.to_string());
                match Self::block_on(self.store.set_status_color(status_id, color_str.as_deref())) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::ReorderStatus { id, new_position } => {
                match Self::block_on(self.store.reorder_status(self.project_id, id, new_position)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            _ => {}
        }
    }

    fn block_on<T>(f: impl Future<Output = T>) -> T {
        tokio::task::block_in_place(|| Handle::current().block_on(f))
    }
}
