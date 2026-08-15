use crate::models::TaskID;
use crate::tui::component::task_status_list::TaskStatusList;
use crate::tui::component::{ProjectStatusTasks, RenderContext};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use std::iter;

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
        project_status_tasks: &ProjectStatusTasks,
        selected_task_id: Option<TaskID>,
    ) {
        self.adjust_scroll_offset_for_selected_task(
            ctx.area,
            project_status_tasks,
            selected_task_id,
        );

        let task_list_lines: Vec<Line> = project_status_tasks
            .status_tasks
            .iter()
            .flat_map(|status_tasks| {
                iter::once(Line::default()).chain(Vec::from(TaskStatusList::new(
                    status_tasks,
                    selected_task_id,
                )))
            })
            .skip(1) // drop the leading blank before the first status
            .collect();

        ctx.render(Paragraph::new(task_list_lines).scroll((self.scroll_offset, 0)));
    }

    fn line_index_from_task_id(project_status_tasks: &ProjectStatusTasks, task_id: TaskID) -> u16 {
        let mut line_index = 0;
        for status in &project_status_tasks.status_tasks {
            // the status header counts as a line
            line_index += 1;
            for task in &status.tasks {
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
        project_status_tasks: &ProjectStatusTasks,
        selected_task_id: Option<TaskID>,
    ) {
        // todo
        if let Some(task_id) = selected_task_id {
            let line_to_ensure_visibility =
                Self::line_index_from_task_id(project_status_tasks, task_id);

            let min_visible_index = self.scroll_offset;
            let max_visible_index = min_visible_index + area.height - 1;

            if line_to_ensure_visibility < min_visible_index {
                self.scroll_offset -= min_visible_index - line_to_ensure_visibility
            } else if line_to_ensure_visibility > max_visible_index {
                self.scroll_offset += line_to_ensure_visibility - max_visible_index
            }

            if line_to_ensure_visibility == self.scroll_offset
                && let Some(iis) = project_status_tasks.index_in_status(task_id)
                && iis == 0
            {
                // header visibility!
                self.scroll_offset -= 1
            }
        }
    }
}
