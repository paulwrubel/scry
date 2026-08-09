use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::models::State;
use crate::models::Task;
use crate::tui::action::Action;
use crate::tui::component::{AppContext, Component};

pub struct TaskList {
    // ── internal ──
    // which visible row is selected (tasks only; input bar selection is handled by the parent)
    selected_index: usize,
}

impl TaskList {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    fn build_visual_order(states: &[State], tasks: &[Task]) -> Vec<usize> {
        let mut order = Vec::new();
        for state in states {
            for (task_idx, task) in tasks.iter().enumerate() {
                if task.state_id == state.id {
                    order.push(task_idx);
                }
            }
        }
        order
    }

    pub fn selected_task_id(&self, ctx: &AppContext) -> Option<i64> {
        let order = Self::build_visual_order(ctx.states, ctx.tasks);
        if self.selected_index < order.len() {
            Some(ctx.tasks[order[self.selected_index]].id)
        } else {
            None
        }
    }

    fn compute_vertical_scroll_offset(&self, ctx: &AppContext, viewport_height: u16) -> u16 {
        let order = Self::build_visual_order(ctx.states, ctx.tasks);
        let mut line_index: u16 = 0;

        if self.selected_index >= order.len() {
            // empty task list — scroll past all state headers
            for state in ctx.states {
                line_index += 1;
                let count = ctx.tasks.iter().filter(|t| t.state_id == state.id).count();
                line_index += count as u16;
            }
        } else {
            let flat_idx = order[self.selected_index];
            let selected_state_id = ctx.tasks[flat_idx].state_id;

            for state in ctx.states {
                line_index += 1;

                if state.id == selected_state_id {
                    // count tasks in this state before the selected one
                    let tasks_in_state: Vec<_> = ctx
                        .tasks
                        .iter()
                        .filter(|t| t.state_id == state.id)
                        .collect();
                    let pos = tasks_in_state
                        .iter()
                        .position(|t| t.id == ctx.tasks[flat_idx].id)
                        .unwrap_or(0);
                    line_index += pos as u16;
                    break;
                }

                let count = ctx.tasks.iter().filter(|t| t.state_id == state.id).count();
                line_index += count as u16;
            }
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

    fn render_state_header(state_name: &str, task_count: usize) -> Line<'static> {
        Line::from(Span::styled(
            format!("{} ({}):", state_name, task_count),
            Style::default().add_modifier(Modifier::BOLD),
        ))
    }

    fn render_task_row(
        task: &Task,
        selected: bool,
        is_completed: bool,
        state_color: Option<Color>,
    ) -> Line<'static> {
        let Task { id, title, .. } = task;

        let checkbox = if is_completed { "[x]" } else { "[ ]" };
        let row_text = format!(" {id:>3} {checkbox} {title}");

        let mut style = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        if !selected && let Some(color) = state_color {
            style = style.fg(color);
        }

        Line::styled(row_text, style)
    }
}

impl Component for TaskList {
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action> {
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
                let task_count = ctx.tasks.len();
                if self.selected_index >= task_count.saturating_sub(1) || task_count == 0 {
                    Some(Action::MoveFocusDown)
                } else {
                    self.selected_index += 1;
                    None
                }
            }
            KeyCode::Enter => self.selected_task_id(ctx).map(Action::OpenPopupTaskDetail),
            KeyCode::Char('m') => self.selected_task_id(ctx).map(Action::OpenPopupMovePicker),
            KeyCode::Char('d') => self
                .selected_task_id(ctx)
                .map(Action::OpenPopupDeleteConfirm),
            _ => None,
        }
    }

    fn render(&self, ctx: &AppContext, frame: &mut Frame, area: Rect) {
        let scroll_offset = Self::compute_vertical_scroll_offset(self, ctx, area.height);

        let mut lines: Vec<Line> = Vec::new();
        let selected_id = self.selected_task_id(ctx);

        for (i, state) in ctx.states.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(""));
            }

            let state_tasks: Vec<_> = ctx
                .tasks
                .iter()
                .filter(|t| t.state_id == state.id)
                .collect();

            lines.push(Self::render_state_header(&state.name, state_tasks.len()));

            for task in state_tasks {
                let selected = Some(task.id) == selected_id;
                lines.push(Self::render_task_row(
                    task,
                    selected,
                    state.is_completed,
                    state.color.clone().map(Into::into),
                ));
            }
        }

        let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }
}
