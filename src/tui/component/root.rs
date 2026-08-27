use crate::models::{Task, TaskID};
use crate::tui::action::Action;
use crate::tui::component::popup::{AddOrEditTask, ConfirmDelete, ConfirmDeleteEntity, ErrorInfo};
use crate::tui::component::{
    CommandInput, Hints, Popup, ProjectStatusTasks, RenderContext, State, TaskDetails, TaskList,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Spacing};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

#[derive(Debug, Clone, Copy)]
enum SelectedTask {
    First,
    Last,
    ID(TaskID),
}

pub struct Root {
    task_list: TaskList,
    command_input: CommandInput,
    hints: Hints,
    popup: Option<Popup>,

    selected_task: Option<SelectedTask>,
}

impl Root {
    pub fn new() -> Self {
        Self {
            task_list: TaskList::new(true),
            command_input: CommandInput::new(false),
            hints: Hints::new(),
            popup: None,

            selected_task: Some(SelectedTask::First),
        }
    }

    fn handle_action(&mut self, state: &State, action: Action) -> Vec<Action> {
        self.handle_actions(state, vec![action])
    }

    fn handle_actions(&mut self, state: &State, actions: Vec<Action>) -> Vec<Action> {
        actions
            .into_iter()
            .flat_map(|action| {
                match action {
                    // UI actions — handled here, never reach the coordinator
                    Action::OpenPopupConfirmDelete(entity) => {
                        self.popup = Some(Popup::ConfirmDelete(ConfirmDelete::new(entity)));

                        vec![]
                    }
                    Action::OpenPopupCreateTask(task) => {
                        self.popup = Some(Popup::AddOrEditTask(AddOrEditTask::new(task)));
                        vec![]
                    }
                    Action::OpenPopupErrorInfo(error_text) => {
                        self.popup = Some(Popup::ErrorInfo(ErrorInfo::new(error_text)));
                        vec![]
                    }
                    Action::DismissPopup => {
                        self.popup = None;
                        vec![]
                    }

                    Action::CloseCommandInput => {
                        self.command_input.reset();
                        self.command_input.blur();
                        vec![]
                    }

                    // Store actions that also dismiss the popup before bubbling
                    Action::CreateTask(_)
                    | Action::UpdateTask(_)
                    | Action::CreateStatus(_)
                    | Action::UpdateStatus(_)
                    | Action::DeleteStatus(_) => {
                        self.popup = None;
                        vec![action]
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
                            } else if let Some(previous) =
                                project_status_tasks.previous_task(task_id)
                            {
                                self.selected_task = Some(SelectedTask::ID(previous.id))
                            } else {
                                self.selected_task = Some(SelectedTask::First)
                            }
                        }

                        vec![action]
                    }

                    // unhandled actions bubble up unchanged
                    _ => vec![action],
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Vec<Action> {
        let code = key.code;

        // if we get a quit request, we do that, regardless of anything else
        if let (KeyModifiers::CONTROL, KeyCode::Char('c')) = (key.modifiers, code) {
            return self.handle_action(state, Action::Quit);
        }

        // active popup swallows all input
        if let Some(ref mut popup) = self.popup {
            return popup
                .handle_event(state, key)
                .map_or(vec![], |a| self.handle_action(state, a));
        }

        // if the command input is focused, send the event there
        if self.command_input.is_focused {
            let actions = self.command_input.handle_event(state, key);
            return self.handle_actions(state, actions);
        }

        // global keys are handled ONLY if nothing above handled the event
        let project_status_tasks: ProjectStatusTasks = state.into();
        let selected = self.task_from_selected_task(&project_status_tasks);
        match (key.modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.command_input.focus();

                vec![]
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.handle_action(state, Action::OpenPopupCreateTask(None))
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                if let Some(task) = selected {
                    self.handle_action(state, Action::OpenPopupCreateTask(Some(task.clone())))
                } else {
                    vec![]
                }
            }
            (_, KeyCode::Char('d')) if self.task_list.is_focused => {
                if let Some(task) = selected {
                    self.handle_action(
                        state,
                        Action::OpenPopupConfirmDelete(ConfirmDeleteEntity::Task(task.clone())),
                    )
                } else {
                    vec![]
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
                previous_status.map_or(vec![], |status| {
                    self.handle_action(
                        state,
                        Action::UpdateTask(Task {
                            status_id: status.id,
                            ..task.clone()
                        }),
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
                next_status.map_or(vec![], |status| {
                    self.handle_action(
                        state,
                        Action::UpdateTask(Task {
                            status_id: status.id,
                            ..task.clone()
                        }),
                    )
                })
            }
            (modifier, KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K'))
                if self.task_list.is_focused =>
            {
                // first, check if anything at all is selected
                if let Some(task) = self.task_from_selected_task(&project_status_tasks) {
                    // if the user is holding shift, just go to the top
                    if modifier == KeyModifiers::SHIFT {
                        self.selected_task = Some(SelectedTask::First)
                    // otherwise, if there's a previous task, select it
                    } else if let Some(prev) = project_status_tasks.previous_task(task.id) {
                        self.selected_task = Some(SelectedTask::ID(prev.id));
                    }
                }

                vec![]
            }
            (modifier, KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J'))
                if self.task_list.is_focused =>
            {
                // first, check if anything at all is selected
                if let Some(task) = self.task_from_selected_task(&project_status_tasks) {
                    // if the user is holding shift, just go to the top
                    if modifier == KeyModifiers::SHIFT {
                        self.selected_task = Some(SelectedTask::Last)
                    // otherwise, if there's a previous task, select it
                    } else if let Some(next) = project_status_tasks.next_task(task.id) {
                        self.selected_task = Some(SelectedTask::ID(next.id));
                    }
                }

                vec![]
            }
            _ => vec![],
        }
    }

    pub fn render(&mut self, ctx: &mut RenderContext) {
        // main pane and hints
        let [task_area, command_input_area, hints_area] = ctx.area.layout(
            &Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(if self.command_input.is_focused { 3 } else { 1 }),
                Constraint::Length(3),
            ])
            .spacing(Spacing::Overlap(1)),
        );
        let [task_list_area, task_details_area] = task_area.layout(
            &Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Fill(if task_area.width >= 80 { 1 } else { 0 }),
            ])
            .spacing(Spacing::Overlap(if task_area.width >= 80 { 1 } else { 0 })),
        );

        // create the base bordered block
        let block = Block::default()
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact);

        // render the task list content
        //
        // todo: move this padding stuff unti the actual task_list component
        let horizontal_padding = (1, 0);
        let vertical_padding = (1, 0);
        let [_, task_list_content_area, _] =
            block.inner(task_list_area).layout(&Layout::horizontal([
                Constraint::Length(horizontal_padding.0), // padding
                Constraint::Min(0),                       // content
                Constraint::Length(horizontal_padding.1), // padding
            ]));
        let [_, task_list_content_area, _] = task_list_content_area.layout(&Layout::vertical([
            Constraint::Length(vertical_padding.0), // padding
            Constraint::Min(0),                     // content
            Constraint::Length(vertical_padding.1), // padding
        ]));

        let project_status_tasks: ProjectStatusTasks = ctx.state.into();
        let selected_task = self.task_from_selected_task(&project_status_tasks);
        self.task_list.render(
            &mut ctx.with_area(task_list_content_area),
            &project_status_tasks,
            selected_task.map(|t| t.id),
        );
        // task list block
        let project_name = &ctx.state.project.name;
        ctx.with_area(task_list_area).render(
            block
                .clone()
                .title(Line::from(format!(" {} ", project_name)).centered()),
        );

        // task details
        let task_details_content_area = block.inner(task_details_area);
        if let Some(selected_task) = selected_task {
            TaskDetails::new(selected_task).render(&mut ctx.with_area(task_details_content_area));
        }
        // task details block
        ctx.with_area(task_details_area)
            .render(block.clone().title(Line::from(" Task Details ").centered()));

        // command input area
        if self.command_input.is_focused {
            let [command_input_content_area] = block
                .inner(command_input_area)
                .layout(&Layout::horizontal([Constraint::Min(0)]).horizontal_margin(1));
            self.command_input
                .render(&mut ctx.with_area(command_input_content_area));
            // command input block
            ctx.with_area(command_input_area)
                .render(block.clone().title(Line::from(" command ")));
        }

        // hints border and content
        let [hints_content_area] = block
            .inner(hints_area)
            .layout(&Layout::horizontal([Constraint::Min(0)]).horizontal_margin(1));
        self.hints.render(
            &mut ctx.with_area(hints_content_area),
            selected_task.map(|t| t.id),
        );
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
                SelectedTask::Last => project_status_tasks.last(),
                SelectedTask::ID(task_id) => project_status_tasks.get_task_by_id(task_id),
            },
            None => None,
        }
    }
}
