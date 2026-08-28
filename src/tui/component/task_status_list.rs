use crate::{
    models::TaskID,
    tui::{
        component::{TaskLine, shared::truncate_string_to_width},
        state::StatusWithTasks,
    },
};
use ratatui::{
    style::{Color, Stylize},
    text::{Line, Span, Text},
};

pub struct TaskStatusList<'a> {
    status_with_tasks: &'a StatusWithTasks,
    selected_task_id: Option<TaskID>,
    area_width: u16,
}

impl<'a> TaskStatusList<'a> {
    pub fn new(
        status_with_tasks: &'a StatusWithTasks,
        selected_task_id: Option<TaskID>,
        area_width: u16,
    ) -> Self {
        Self {
            status_with_tasks,
            selected_task_id,
            area_width,
        }
    }
}

impl<'a> From<TaskStatusList<'a>> for Text<'a> {
    fn from(value: TaskStatusList<'a>) -> Self {
        let status: &crate::models::Status = &value.status_with_tasks.status;
        let status_name = &status.name;
        let task_count = value.status_with_tasks.tasks_with_notes.len();

        let status_color = status.color.map_or(Color::default(), |c| c.into());

        let mut text = Text::default();

        let task_count_str = format!("[{task_count}]");
        let status_name_str = truncate_string_to_width(
            status_name.clone(),
            value
                .area_width
                .saturating_sub(task_count_str.chars().count() as u16)
                .saturating_sub(1)
                .into(),
        );
        text.push_line(
            Line::from(vec![
                Span::from(status_name_str.clone()).italic(),
                Span::from(
                    " ".repeat(
                        value
                            .area_width
                            .saturating_sub(
                                // status name is truncated above, there's no risk here
                                status_name_str.chars().count() as u16
                                // and for task count, this is essentially statically bounded
                                    + task_count_str.chars().count() as u16,
                            )
                            .into(),
                    ),
                ),
                Span::from(task_count_str),
            ])
            .fg(status_color),
        );
        text.push_line(
            Line::from("─".repeat(value.area_width.into()))
                .fg(status_color)
                .dim(),
        );

        for task_text in value.status_with_tasks.tasks_with_notes.iter().map(|task| {
            Text::from(Line::from(TaskLine::new(
                task.clone(),
                None,
                status.style,
                Some(task.id) == value.selected_task_id,
                value.area_width,
            )))
        }) {
            text += task_text;
        }

        text
    }
}
