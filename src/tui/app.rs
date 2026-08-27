use crate::models::ProjectID;
use crate::store::TaskStore;
use crate::tui::action::Action;
use crate::tui::component::RenderContext;
use crate::tui::component::Root;
use crate::{
    error::{AppError, StorageError},
    tui::component::State,
};
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

            // popup lifecycle and command input are handled internally by Root
            Action::OpenPopupConfirmDelete(_) => {}
            Action::OpenPopupCreateTask(_) => {}
            Action::OpenPopupErrorInfo(_) => {}
            Action::DismissPopup => {}
            Action::CloseCommandInput => {}

            Action::CreateTask(task) => {
                match Self::block_on(self.store.create_task(
                    task.project_id,
                    task.title,
                    task.description,
                    task.status_id,
                    task.position,
                )) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::UpdateTask(task) => {
                match Self::block_on(self.store.update_and_autoposition_task(task)) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::DeleteTask(task_id) => match Self::block_on(self.store.delete_task(task_id)) {
                Ok(_) => {}
                Err(e) => self.root.set_status(e.to_string()),
            },
            Action::CreateStatus(status) => {
                match Self::block_on(self.store.create_status(
                    status.project_id,
                    status.name,
                    status.position,
                    status.color,
                    status.style,
                )) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::UpdateStatus(status) => {
                match Self::block_on(async {
                    let current =
                        self.store
                            .get_status_by_id(status.id)
                            .await?
                            .ok_or_else(|| {
                                StorageError::NotFound(format!(
                                    "status with id '{}' not found",
                                    status.id
                                ))
                            })?;
                    if current.position == status.position {
                        self.store.update_status(status).await.map(|_| ())
                    } else {
                        self.store
                            .reorder_status(status.project_id, status.id, status.position)
                            .await
                    }
                }) {
                    Ok(_) => {}
                    Err(e) => self.root.set_status(e.to_string()),
                }
            }
            Action::DeleteStatus(id) => match Self::block_on(self.store.delete_status(id)) {
                Ok(_) => {}
                Err(e) => self.root.set_status(e.to_string()),
            },
        }
    }

    fn block_on<T>(f: impl Future<Output = T>) -> T {
        tokio::task::block_in_place(|| Handle::current().block_on(f))
    }
}
