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
use std::time::Duration;
use tokio::runtime::Handle;

/// Terminal lifecycle and domain logic. UI orchestration lives in Root.
pub struct App<'a> {
    root: Root<'a>,
    is_running: bool,

    // domain state
    _config: ScryConfig,
    store: ArcStore,
    project_id: ProjectId,

    /// Snapshot as last loaded from the store; `None` until the initial load.
    loaded: Option<ProjectState>,
    /// Cached view that rendering and navigation operate on: `loaded` plus the
    /// active filter and hidden-status collapsing. Rebuilt only when those change.
    view: Option<ProjectState>,
    /// Filter `view` was built with, so filter edits can trigger a rebuild without
    /// touching the store.
    view_filter: String,
    /// Set when an action dirtied the snapshot, meaning `loaded` is stale and must
    /// be reloaded before the next draw.
    dirty: bool,
}

impl App<'_> {
    pub fn new(config: ScryConfig, store: ArcStore, project_id: ProjectId) -> Self {
        App {
            root: Root::new(),
            is_running: true,

            _config: config,
            store,
            project_id,

            loaded: None,
            view: None,
            view_filter: String::new(),
            dirty: false,
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

    async fn reload(&mut self) -> Result<(), AppError> {
        self.loaded =
            Some(ProjectState::load_from_store(self.store.as_ref(), self.project_id).await?);
        self.dirty = false;
        self.refresh_view();
        Ok(())
    }

    fn refresh_view(&mut self) {
        let Some(loaded) = self.loaded.clone() else {
            self.view = None;
            return;
        };

        let filter = self.root.current_filter();
        let mut state = loaded;
        if !filter.is_empty() {
            state = state.with_substring_filter(filter.clone());
        }

        self.view = Some(state.with_hidden_statuses_collapsed());
        self.view_filter = filter;
    }

    fn draw(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), AppError> {
        let Some(state) = self.view.take() else {
            return Ok(());
        };

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

        self.view = Some(state);
        Ok(())
    }

    fn handle_input_event(&mut self, event: Event) {
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }

        let Some(state) = self.view.take() else {
            return;
        };

        for action in self.root.handle_event(&state, key) {
            if let Some(action) = self.process_action(&state, action) {
                // popup-opening actions (e.g. error popups) go back through Root,
                // which owns the popup lifecycle
                self.root.handle_action(&state, action);
            }
        }

        self.view = Some(state);

        if self.root.current_filter() != self.view_filter {
            self.refresh_view();
        }
    }

    async fn apply_input(&mut self, events: Vec<Event>) -> Result<(), AppError> {
        // A whole burst of keys is handled against the in-memory snapshot so the
        // store is consulted at most once per pass, after the burst.
        for event in events {
            self.handle_input_event(event);
            if !self.is_running {
                break;
            }
        }

        // A quit in the same burst makes the reload pointless (and could fail on
        // the way out), so only resync while the TUI is still running.
        if self.dirty && self.is_running {
            self.reload().await?;
        }

        Ok(())
    }

    fn read_event() -> Result<Event, AppError> {
        event::read().map_err(|e| AppError::Internal(format!("event error: {}", e)))
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), AppError> {
        self.reload().await?;

        while self.is_running {
            self.draw(terminal)?;

            let mut events = vec![Self::read_event()?];
            while event::poll(Duration::ZERO)
                .map_err(|e| AppError::Internal(format!("event error: {}", e)))?
            {
                events.push(Self::read_event()?);
            }

            self.apply_input(events).await?;
        }
        Ok(())
    }

    fn process_action(&mut self, state: &ProjectState, action: Action) -> Option<Action> {
        // Any action that touches the store invalidates the cached snapshot. Marking
        // dirty up front means a write that partially applied is resynced from the
        // store even when the call then reports an error.
        if action.is_mutable() {
            self.dirty = true;
        }

        let service = ProjectService::new(self.store.as_ref());
        match action {
            Action::Quit => {
                self.is_running = false;
                None
            }

            // the reload itself is deferred to the end of the input pass
            Action::Refresh => None,

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
            Action::UpdateProject(update) => {
                match Self::block_on(service.update_project(state.project(), update)) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use scry_core::backup::{ImportPlan, ImportReport};
    use scry_core::error::StorageError;
    use scry_core::models::{
        Color, Note, NoteId, Priority, Project, Status, StatusId, StatusStyle, Tags, Task, TaskId,
        TaskSortingMode,
    };
    use scry_core::store::{ProjectSnapshot, Store, TaskToCreate};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingStore {
        snapshot: ProjectSnapshot,
        snapshot_loads: AtomicUsize,
    }

    impl CountingStore {
        fn new() -> Self {
            Self {
                snapshot: ProjectSnapshot {
                    project: Project {
                        id: 1,
                        name: "test".to_string(),
                        description: None,
                        entry_status_id: None,
                        task_sorting_mode: TaskSortingMode::Manual,
                        show_priority: false,
                        created_at: Utc::now(),
                    },
                    statuses: vec![Status {
                        id: 1,
                        project_id: 1,
                        name: "todo".to_string(),
                        position: 0,
                        color: None,
                        style: StatusStyle::None,
                    }],
                    tasks: vec![Task {
                        id: 1,
                        project_id: 1,
                        title: "task 1".to_string(),
                        description: None,
                        priority: Priority::default(),
                        status_id: 1,
                        position: 0,
                        tags: Tags::default(),
                        created_at: Utc::now(),
                    }],
                    notes: Vec::new(),
                },
                snapshot_loads: AtomicUsize::new(0),
            }
        }

        fn snapshot_loads(&self) -> usize {
            self.snapshot_loads.load(Ordering::SeqCst)
        }
    }

    // only `get_project_snapshot` is reachable from the TUI paths under test, so
    // every other method stays unimplemented
    #[async_trait::async_trait]
    impl Store for CountingStore {
        async fn create_task(&self, _task_to_create: TaskToCreate) -> Result<Task, StorageError> {
            unimplemented!()
        }

        async fn get_task_by_id(&self, _id: TaskId) -> Result<Option<Task>, StorageError> {
            unimplemented!()
        }

        async fn get_all_tasks_by_project_id(
            &self,
            _project_id: ProjectId,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!()
        }

        async fn get_all_tasks_by_status_id(
            &self,
            _status_id: StatusId,
        ) -> Result<Vec<Task>, StorageError> {
            unimplemented!()
        }

        async fn update_task(&self, _task: Task) -> Result<Task, StorageError> {
            unimplemented!()
        }

        async fn update_and_autoposition_task(&self, _task: Task) -> Result<Task, StorageError> {
            unimplemented!()
        }

        async fn delete_task(&self, _id: TaskId) -> Result<(), StorageError> {
            unimplemented!()
        }

        async fn create_note(
            &self,
            _task_id: TaskId,
            _contents: String,
        ) -> Result<Note, StorageError> {
            unimplemented!()
        }

        async fn get_note_by_id(&self, _id: NoteId) -> Result<Option<Note>, StorageError> {
            unimplemented!()
        }

        async fn get_all_notes_by_task_id(
            &self,
            _task_id: TaskId,
        ) -> Result<Vec<Note>, StorageError> {
            unimplemented!()
        }

        async fn get_all_notes_by_project_id(
            &self,
            _project_id: ProjectId,
        ) -> Result<Vec<Note>, StorageError> {
            unimplemented!()
        }

        async fn update_note(&self, _note: Note) -> Result<Note, StorageError> {
            unimplemented!()
        }

        async fn delete_note(&self, _id: NoteId) -> Result<(), StorageError> {
            unimplemented!()
        }

        async fn create_status(
            &self,
            _project_id: ProjectId,
            _name: String,
            _position: i32,
            _color: Option<Color>,
            _style: StatusStyle,
        ) -> Result<Status, StorageError> {
            unimplemented!()
        }

        async fn get_status_by_id(&self, _id: StatusId) -> Result<Option<Status>, StorageError> {
            unimplemented!()
        }

        async fn get_status_by_project_id_and_status_name(
            &self,
            _project_id: ProjectId,
            _status_name: String,
        ) -> Result<Option<Status>, StorageError> {
            unimplemented!()
        }

        async fn get_all_statuses_by_project_id(
            &self,
            _project_id: ProjectId,
        ) -> Result<Vec<Status>, StorageError> {
            unimplemented!()
        }

        async fn update_status(&self, _status: Status) -> Result<Status, StorageError> {
            unimplemented!()
        }

        async fn reorder_status(
            &self,
            _project_id: ProjectId,
            _status_id: StatusId,
            _new_position: i32,
        ) -> Result<(), StorageError> {
            unimplemented!()
        }

        async fn delete_status(&self, _id: StatusId) -> Result<(), StorageError> {
            unimplemented!()
        }

        async fn create_project(
            &self,
            _name: String,
            _description: Option<String>,
            _entry_status_id: Option<StatusId>,
            _task_sorting_mode: TaskSortingMode,
            _show_priority: bool,
        ) -> Result<Project, StorageError> {
            unimplemented!()
        }

        async fn get_project_by_id(&self, _id: ProjectId) -> Result<Option<Project>, StorageError> {
            unimplemented!()
        }

        async fn get_project_snapshot(
            &self,
            _project_id: ProjectId,
        ) -> Result<ProjectSnapshot, StorageError> {
            self.snapshot_loads.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }

        async fn get_project_by_name(&self, _name: &str) -> Result<Option<Project>, StorageError> {
            unimplemented!()
        }

        async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError> {
            unimplemented!()
        }

        async fn update_project(&self, _project: Project) -> Result<Project, StorageError> {
            unimplemented!()
        }

        async fn delete_project(&self, _name: String) -> Result<(), StorageError> {
            unimplemented!()
        }

        async fn import(&self, _plan: ImportPlan) -> Result<ImportReport, StorageError> {
            unimplemented!()
        }
    }

    fn test_app(store: &Arc<CountingStore>) -> App<'static> {
        let store: ArcStore = Arc::<CountingStore>::clone(store);
        App::new(
            ScryConfig {
                database_url: "sqlite://:memory:".to_string(),
                state_file: None,
            },
            store,
            1,
        )
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[tokio::test]
    async fn non_mutating_events_do_not_touch_the_store() {
        let store = Arc::new(CountingStore::new());
        let mut app = test_app(&store);

        app.reload().await.expect("initial load");
        let baseline = store.snapshot_loads();
        assert_eq!(baseline, 1, "initial reload reads the store once");

        // navigation, command input open/close, and filter focus/typing/clear in
        // one burst, none of which should mark the snapshot dirty
        let events = vec![
            key(KeyCode::Down),
            key(KeyCode::Char('j')),
            key(KeyCode::Up),
            key(KeyCode::Char('k')),
            key(KeyCode::Char('/')),
            key(KeyCode::Esc),
            key(KeyCode::Char('f')),
            key(KeyCode::Char('x')),
            key(KeyCode::Backspace),
            key(KeyCode::Esc),
            key(KeyCode::Down),
            key(KeyCode::Char('j')),
        ];
        app.apply_input(events).await.expect("non-mutating input");

        assert_eq!(
            store.snapshot_loads(),
            baseline,
            "non-mutating input must not read the store"
        );
        assert!(app.view.is_some(), "navigation keeps the cached view");

        app.apply_input(vec![key(KeyCode::Esc), key(KeyCode::Down)])
            .await
            .expect("second non-mutating burst");

        assert_eq!(
            store.snapshot_loads(),
            baseline,
            "a second non-mutating burst must not read the store"
        );
    }

    #[tokio::test]
    async fn coalesced_refresh_burst_reloads_exactly_once() {
        let store = Arc::new(CountingStore::new());
        let mut app = test_app(&store);

        app.reload().await.expect("initial load");
        assert_eq!(store.snapshot_loads(), 1);

        app.apply_input(vec![
            key(KeyCode::Down),
            key(KeyCode::Char('r')),
            key(KeyCode::Down),
            key(KeyCode::Char('r')),
        ])
        .await
        .expect("refresh burst");

        assert_eq!(
            store.snapshot_loads(),
            2,
            "two refresh presses in one burst reload once"
        );

        app.apply_input(vec![key(KeyCode::Down), key(KeyCode::Char('j'))])
            .await
            .expect("follow-up burst");

        assert_eq!(
            store.snapshot_loads(),
            2,
            "a clean burst must not reload the snapshot"
        );
    }
}
