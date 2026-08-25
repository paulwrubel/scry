use crate::{
    models::TaskID,
    tui::component::{StatusTasks, TaskLine},
};
use ratatui::{style::Style, text::Line};

pub struct TaskStatusList<'a> {
    status_tasks: &'a StatusTasks,
    selected_task_id: Option<TaskID>,
}

impl<'a> TaskStatusList<'a> {
    pub fn new(status_tasks: &'a StatusTasks, selected_task_id: Option<TaskID>) -> Self {
        Self {
            status_tasks,
            selected_task_id,
        }
    }
}

impl<'a> From<TaskStatusList<'a>> for Vec<Line<'a>> {
    fn from(value: TaskStatusList<'a>) -> Self {
        let status_name = &value.status_tasks.status.name;
        let task_count = value.status_tasks.tasks.len();

        let header_line = Line::styled(
            format!("{status_name} ({task_count}):"),
            Style::default().underlined().bold(),
        );

        let status = &value.status_tasks.status;
        let status_color = status.color.map(Into::into).unwrap_or_default();
        let task_lines = value.status_tasks.tasks.iter().map(|task| {
            TaskLine::new(
                task,
                status_color,
                status.style,
                Some(task.id) == value.selected_task_id,
            )
            .into()
        });

        vec![header_line].into_iter().chain(task_lines).collect()
    }
}
