use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::models::Status;
use crate::models::Task;
use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};

pub struct TaskList {
    // which visible row is selected (tasks only)
    selected_index: usize,
}

impl TaskList {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    fn build_visual_order(statuses: &[Status], tasks: &[Task]) -> Vec<usize> {
        let mut order = Vec::new();
        for status in statuses {
            for (task_idx, task) in tasks.iter().enumerate() {
                if task.status_id == status.id {
                    order.push(task_idx);
                }
            }
        }
        // ── internal ──
        order
    }

    pub fn selected_task_id(&self, state: &State) -> Option<i64> {
        let order = Self::build_visual_order(&state.statuses, &state.tasks);
        if self.selected_index < order.len() {
            Some(state.tasks[order[self.selected_index]].id)
        } else {
            None
        }
    }

    fn compute_vertical_scroll_offset(&self, state: &State, viewport_height: u16) -> u16 {
        let order = Self::build_visual_order(&state.statuses, &state.tasks);
        if order.is_empty() {
            return 0;
        }

        let selected_index = self.selected_index.clamp(0, order.len().saturating_sub(1));

        let mut line_index: u16 = 0;

        let flat_idx = order[selected_index];
        let selected_status_id = state.tasks[flat_idx].status_id;

        for status in &state.statuses {
            // blank separator between status groups (matches render)
            if line_index > 0 {
                line_index += 1;
            }
            line_index += 1;

            if status.id == selected_status_id {
                // count tasks in this status before the selected one
                let tasks_in_status: Vec<_> = state
                    .tasks
                    .iter()
                    .filter(|t| t.status_id == status.id)
                    .collect();
                let pos = tasks_in_status
                    .iter()
                    .position(|t| t.id == state.tasks[flat_idx].id)
                    .unwrap_or(0);
                line_index += pos as u16;
                break;
            }

            let count = state
                .tasks
                .iter()
                .filter(|t| t.status_id == status.id)
                .count();
            line_index += count as u16;
        }

        // ensure the selected line is visible within the viewport
        if viewport_height == 0 {
            return 0;
        }
        if line_index >= viewport_height {
            line_index - viewport_height + 1
        } else {
            0
        }
    }

    fn render_status_header(status_name: &str, task_count: usize) -> Line<'static> {
        Line::from(Span::styled(
            format!("{status_name} ({task_count}):"),
            Style::default().add_modifier(Modifier::BOLD),
        ))
    }

    fn render_task_row(
        task: &Task,
        selected: bool,
        is_completed: bool,
        status_color: Option<Color>,
    ) -> Line<'static> {
        let Task { id, title, .. } = task;

        let checkbox = if is_completed { "[x]" } else { "[ ]" };
        let row_text = format!(" {id:>3} {checkbox} {title}");

        let mut style = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        if !selected && let Some(color) = status_color {
            style = style.fg(color);
        }

        Line::styled(row_text, style)
    }

    pub fn handle_event(&mut self, state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index == 0 {
                    Some(Action::MoveFocusUp)
                } else {
                    self.selected_index -= 1;
                    None
                }
            }
            KeyCode::Down => {
                let task_count = state.tasks.len();
                if self.selected_index >= task_count.saturating_sub(1) || task_count == 0 {
                    Some(Action::MoveFocusDown)
                } else {
                    self.selected_index += 1;
                    None
                }
            }
            KeyCode::Enter => self
                .selected_task_id(state)
                .map(Action::OpenPopupTaskDetail),
            KeyCode::Char('m') => self
                .selected_task_id(state)
                .map(Action::OpenPopupMovePicker),
            KeyCode::Char('d') => self
                .selected_task_id(state)
                .map(Action::OpenPopupDeleteConfirm),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let scroll_offset = Self::compute_vertical_scroll_offset(self, ctx.state, ctx.area.height);

        let mut lines: Vec<Line> = Vec::new();
        let selected_id = self.selected_task_id(ctx.state);

        for (i, status) in ctx.state.statuses.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(""));
            }

            let status_tasks: Vec<_> = ctx
                .state
                .tasks
                .iter()
                .filter(|t| t.status_id == status.id)
                .collect();

            lines.push(Self::render_status_header(&status.name, status_tasks.len()));

            for task in status_tasks {
                let selected = Some(task.id) == selected_id;
                lines.push(Self::render_task_row(
                    task,
                    selected,
                    status.is_completed,
                    status.color.clone().map(Into::into),
                ));
            }
        }

        ctx.render_widget(Paragraph::new(lines).scroll((scroll_offset, 0)));
    }
}
