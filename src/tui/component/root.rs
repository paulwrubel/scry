use crate::models::{Task, TaskID};
use crate::tui::action::Action;
use crate::tui::component::popup::{ConfirmDelete, CreateTask};
use crate::tui::component::{
    Hints, Popup, ProjectStatusTasks, RenderContext, State, TaskDetails, TaskList,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Spacing};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

#[derive(Debug, Clone, Copy)]
enum SelectedTask {
    First,
    ID(TaskID),
}

pub struct Root {
    task_list: TaskList,
    hints: Hints,
    popup: Option<Popup>,

    selected_task: Option<SelectedTask>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            task_list: TaskList::new(true),
            hints: Hints::new(),
            popup: None,

            selected_task: Some(SelectedTask::First),
        }
    }

    fn handle_action(&mut self, state: &State, action: Action) -> Option<Action> {
        match action {
            // UI actions — handled here, never reach the coordinator
            Action::OpenPopupDeleteConfirm(task_id) => {
                if let Some(task) = state.tasks.iter().find(|t| t.id == task_id) {
                    self.popup = Some(Popup::ConfirmDelete(ConfirmDelete::new(
                        task_id,
                        task.title.clone(),
                    )));
                }
                None
            }
            Action::OpenPopupCreateTask => {
                self.popup = Some(Popup::CreateTask(CreateTask::new()));
                None
            }
            Action::DismissPopup => {
                self.popup = None;
                None
            }
            // Action::MoveFocusUp | Action::MoveFocusDown => None,

            // Store actions that also dismiss the popup before bubbling
            Action::MoveTask { .. } => {
                self.popup = None;
                Some(action)
            }
            Action::AddTask(_) => {
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
                    if let Some(next) = project_status_tasks.next_task(task_id) {
                        self.selected_task = Some(SelectedTask::ID(next.id))
                    } else if let Some(previous) = project_status_tasks.previous_task(task_id) {
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

        // if we get a quit request, we do that, regardless of anything else
        if let (KeyModifiers::CONTROL, KeyCode::Char('c')) = (key.modifiers, code) {
            return self.handle_action(state, Action::Quit);
        }

        // active popup swallows all input
        if let Some(ref mut popup) = self.popup {
            return popup
                .handle_event(state, key)
                .and_then(|a| self.handle_action(state, a));
        }

        // global keys are handled ONLY if nothing above handled the event

        let project_status_tasks: ProjectStatusTasks = state.into();
        let selected = self.task_from_selected_task(&project_status_tasks);
        match (key.modifiers, code) {
            (_, KeyCode::Char('d')) if self.task_list.is_focused => {
                if let Some(task) = selected {
                    self.handle_action(state, Action::OpenPopupDeleteConfirm(task.id))
                } else {
                    None
                }
            }
            (_, KeyCode::Char(',') | KeyCode::Char('<')) if let Some(task) = selected => {
                // statuses is ordered by position; take the one after the task's current status
                let previous_status = state
                    .statuses
                    .iter()
                    .position(|status| status.id == task.status_id)
                    .and_then(|index| state.statuses.get(index - 1));

                // nothing to move to if the task is already in the last status
                previous_status.and_then(|status| {
                    self.handle_action(
                        state,
                        Action::MoveTask {
                            task_id: task.id,
                            status_id: status.id,
                        },
                    )
                })
            }
            (_, KeyCode::Char('.') | KeyCode::Char('>')) if let Some(task) = selected => {
                // statuses is ordered by position; take the one after the task's current status
                let next_status = state
                    .statuses
                    .iter()
                    .position(|status| status.id == task.status_id)
                    .and_then(|index| state.statuses.get(index + 1));

                // nothing to move to if the task is already in the last status
                next_status.and_then(|status| {
                    self.handle_action(
                        state,
                        Action::MoveTask {
                            task_id: task.id,
                            status_id: status.id,
                        },
                    )
                })
            }
            (_, KeyCode::Up | KeyCode::Char('k')) if self.task_list.is_focused => {
                // check if we have a current selection AND if there's a "previous task"
                if let Some(next_task) = self
                    .task_from_selected_task(&project_status_tasks)
                    .and_then(|task| project_status_tasks.previous_task(task.id))
                {
                    self.selected_task = Some(SelectedTask::ID(next_task.id));
                }

                None
            }
            (_, KeyCode::Down | KeyCode::Char('j')) if self.task_list.is_focused => {
                // check if we have a current selection AND if there's a "next task"
                if let Some(next_task) = self
                    .task_from_selected_task(&project_status_tasks)
                    .and_then(|task| project_status_tasks.next_task(task.id))
                {
                    self.selected_task = Some(SelectedTask::ID(next_task.id));
                }
                None
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.handle_action(state, Action::OpenPopupCreateTask)
            }
            _ => None,
        }
    }

    pub fn render(&mut self, ctx: &mut RenderContext) {
        // main pane and hints
        let [task_area, hints_area] = ctx.area.layout(
            &Layout::vertical([Constraint::Fill(1), Constraint::Length(3)])
                .spacing(Spacing::Overlap(1)),
        );
        let [task_list_area, task_details_area] = task_area
            .layout(&Layout::horizontal([Constraint::Fill(1); 2]).spacing(Spacing::Overlap(1)));

        // create the base bordered block
        let block = Block::default()
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact);

        // render the task list content
        let horizontal_padding = (1, 0);
        let vertical_padding = (1, 0);
        let [_, task_list_content_area, _] = Layout::horizontal([
            Constraint::Length(horizontal_padding.0), // padding
            Constraint::Min(0),                       // content
            Constraint::Length(horizontal_padding.1), // padding
        ])
        .areas(block.inner(task_list_area));
        let [_, task_list_content_area, _] = Layout::vertical([
            Constraint::Length(vertical_padding.0), // padding
            Constraint::Min(0),                     // content
            Constraint::Length(vertical_padding.1), // padding
        ])
        .areas(task_list_content_area);

        let project_status_tasks: ProjectStatusTasks = ctx.state.into();
        let selected_task = self.task_from_selected_task(&project_status_tasks);
        self.task_list.render(
            &mut RenderContext {
                state: ctx.state,
                frame: ctx.frame,
                area: task_list_content_area,
            },
            &project_status_tasks,
            selected_task.map(|t| t.id),
        );

        // task details
        let task_details_content_area = block.inner(task_details_area);
        if let Some(selected_task) = selected_task {
            TaskDetails::new(selected_task).render(&mut RenderContext {
                state: ctx.state,
                frame: ctx.frame,
                area: task_details_content_area,
            });
        }

        // hints border and content
        let [hints_content_area] = block
            .inner(hints_area)
            .layout(&Layout::horizontal([Constraint::Min(0)]).horizontal_margin(1));
        self.hints.render(
            &mut RenderContext {
                state: ctx.state,
                frame: ctx.frame,
                area: hints_content_area,
            },
            selected_task.map(|t| t.id),
        );

        // task list block
        let project_name = &ctx.state.project.name;
        ctx.with_area(task_list_area).render(
            block
                .clone()
                .title(Line::from(format!(" {} ", project_name)).centered()),
        );
        // task details block
        ctx.with_area(task_details_area)
            .render(block.clone().title(Line::from(" Task Details ").centered()));
        // hints block
        ctx.with_area(hints_area)
            .render(block.title_bottom(Line::from(" scry ").right_aligned()));

        // popup last (on top of everything)
        if let Some(ref popup) = self.popup {
            popup.render(ctx);
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.hints.set_message(msg);
    }

    pub fn clear_status(&mut self) {
        self.hints.set_message(String::new());
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
