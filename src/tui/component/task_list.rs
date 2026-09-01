use crate::models::TaskID;
use crate::tui::component::TaskStatusList;
use crate::tui::component::{ProjectState, RenderContext};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

pub struct TaskList {
    pub is_focused: bool,

    scroll_offset: u16,
}

impl TaskList {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,
            scroll_offset: 0,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut RenderContext,
        selected_task_id: Option<TaskID>,
        active_filter: String,
    ) {
        let [task_list_content_area] = ctx
            .area
            .layout(&Layout::horizontal([Constraint::Min(0)]).horizontal_margin(1));
        let [active_filter_area, task_list_content_area, _] =
            task_list_content_area.layout(&Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),    // content
                Constraint::Length(1), // padding
            ]));

        self.adjust_scroll_offset_for_selected_task(
            task_list_content_area,
            ctx.state,
            selected_task_id,
        );

        let task_list_lines: Vec<Line> = itertools::Itertools::intersperse(
            ctx.state.statuses_with_tasks.iter().map(|status_tasks| {
                Text::from(TaskStatusList::new(
                    status_tasks,
                    selected_task_id,
                    ctx.state.project().show_priority,
                    task_list_content_area.width,
                ))
                .lines
            }),
            vec![Line::default()],
        )
        .flatten()
        .collect();

        if !active_filter.is_empty() {
            ctx.with_area(active_filter_area).render(Paragraph::new(
                Line::from(vec![
                    Span::from("filtering for "),
                    Span::from(active_filter).bold(),
                    Span::from(" (Esc to clear)"),
                ])
                .right_aligned(),
            ));
        }
        ctx.with_area(task_list_content_area)
            .render(Paragraph::new(task_list_lines).scroll((self.scroll_offset, 0)));
    }

    fn line_index_from_task_id(state: &ProjectState, task_id: TaskID) -> u16 {
        let mut line_index = 0;
        for status in &state.statuses_with_tasks {
            // the status header counts as a line
            line_index += 1;
            for task in &status.tasks_with_notes {
                if task.id == task_id {
                    return line_index;
                }
                // increment for the not-matching task we just checked
                line_index += 1;
            }
            // spacer after each status section
            line_index += 1;
        }
        panic!("task_id not found in project status tasks!")
    }

    fn adjust_scroll_offset_for_selected_task(
        &mut self,
        area: Rect,
        state: &ProjectState,
        selected_task_id: Option<TaskID>,
    ) {
        // todo
        if let Some(task_id) = selected_task_id {
            let line_to_ensure_visibility = Self::line_index_from_task_id(state, task_id);

            let min_visible_index = self.scroll_offset;
            let max_visible_index = min_visible_index + area.height - 1;

            if line_to_ensure_visibility < min_visible_index {
                self.scroll_offset -= min_visible_index - line_to_ensure_visibility
            } else if line_to_ensure_visibility > max_visible_index {
                self.scroll_offset += line_to_ensure_visibility - max_visible_index
            }

            if line_to_ensure_visibility == self.scroll_offset
                && let Some(iis) = state.index_in_status(task_id)
                && iis == 0
            {
                // header visibility!
                self.scroll_offset -= 1
            }
        }
    }
}
