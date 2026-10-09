use crate::config::ScryConfig;
use crate::error::AppError;
use crate::service::ProjectService;
use crate::state::ProjectState;
use crate::tui::action::Action;
use crate::tui::component::{RenderContext, Root, SelectedTask};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use scry_core::models::ProjectId;
use scry_core::store::ArcStore;
use std::future::Future;
use tokio::runtime::Handle;

/// Terminal lifecycle and domain logic. UI orchestration lives in Root.
pub struct App<'a> {
    root: Root<'a>,
    is_running: bool,

    // domain state
    _config: ScryConfig,
    store: ArcStore,
    project_id: ProjectId,
}

impl App<'_> {
    pub fn new(config: ScryConfig, store: ArcStore, project_id: ProjectId) -> Self {
        App {
            root: Root::new(),
            is_running: true,

            _config: config,
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
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
            SetCursorStyle::BlinkingBlock,
            Show
        )
        .map_err(|e| AppError::Internal(format!("failed to enter alternate screen: {}", e)))?;

        let terminal = Terminal::new(CrosstermBackend::new(stdout))
            .map_err(|e| AppError::Internal(format!("failed to create terminal: {}", e)))?;

        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(
                std::io::stdout(),
                SetCursorStyle::DefaultUserShape,
                PopKeyboardEnhancementFlags,
                LeaveAlternateScreen,
            );
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
            SetCursorStyle::DefaultUserShape,
            PopKeyboardEnhancementFlags,
            LeaveAlternateScreen,
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
            let mut state =
                ProjectState::load_from_store(self.store.as_ref(), self.project_id).await?;

            let filter = self.root.current_filter();
            if !filter.is_empty() {
                state = state.with_substring_filter(filter)
            }

            state = state.with_hidden_statuses_collapsed();

            self.root.bind_selected_task(&state);
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
                    for action in self.root.handle_event(&state, key) {
                        if let Some(action) = self.process_action(&state, action) {
                            // popup-opening actions (e.g. error popups) go back through Root,
                            // which owns the popup lifecycle
                            self.root.handle_action(&state, action);
                        }
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn process_action(&mut self, state: &ProjectState, action: Action) -> Option<Action> {
        let service = ProjectService::new(self.store.as_ref());
        match action {
            Action::Quit => {
                self.is_running = false;
                None
            }

            Action::OpenPopupAddNote(_)
            | Action::OpenPopupAddOrEditTask(_)
            | Action::OpenPopupConfirmDelete(_)
            | Action::OpenPopupErrorInfo(_)
            | Action::DismissPopup
            | Action::CloseCommandInput
            | Action::CloseFilterInput => None,

            Action::CreateTask(input) => {
                match Self::block_on(service.create_task(state.project(), input)) {
                    Ok(change) => {
                        self.root.select_task(SelectedTask::Id(change.task.id));
                        None
                    }
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::UpdateTask { id, input } => {
                match Self::block_on(service.update_task(state.project(), id, input)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::DuplicateTask(id) => {
                match Self::block_on(service.duplicate_task(state.project(), id)) {
                    Ok(change) => {
                        self.root.select_task(SelectedTask::Id(change.task.id));
                        None
                    }
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::DeleteTask(id) => {
                match Self::block_on(service.delete_task(state.project(), id)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::AddTaskNote { task_id, contents } => {
                match Self::block_on(service.add_task_note(state.project(), task_id, contents)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::CreateStatus { name } => {
                match Self::block_on(service.create_status(state.project(), name)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::RenameStatus {
                status_id,
                new_name,
            } => {
                match Self::block_on(service.rename_status(state.project(), status_id, new_name)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::SetStatusColor { status_id, color } => {
                match Self::block_on(service.set_status_color(state.project(), status_id, color)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::SetStatusStyle { status_id, style } => {
                match Self::block_on(service.set_status_style(state.project(), status_id, style)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::MoveStatusUp { status_id } => {
                match Self::block_on(service.move_status_up(state.project(), status_id)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::MoveStatusDown { status_id } => {
                match Self::block_on(service.move_status_down(state.project(), status_id)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::DeleteStatus { status_id } => {
                match Self::block_on(service.delete_status(state.project(), status_id)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::SetProjectEntryStatus { status_id } => {
                match Self::block_on(service.set_project_entry_status(state.project(), status_id)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::SetProjectSortingMode(mode) => {
                match Self::block_on(service.set_project_sorting_mode(state.project(), mode)) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
            Action::SetProjectShouldShowPriority(show) => {
                match Self::block_on(
                    service.set_project_should_show_priority(state.project(), show),
                ) {
                    Ok(_) => None,
                    Err(e) => Some(Action::OpenPopupErrorInfo(e.to_string())),
                }
            }
        }
    }

    fn block_on<T>(f: impl Future<Output = T>) -> T {
        tokio::task::block_in_place(|| Handle::current().block_on(f))
    }
}
