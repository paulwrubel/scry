use crate::models::{self, Task};
use ratatui::{
    style::{Color, Style},
    text::Line,
};

pub struct TaskLine<'a> {
    task: &'a Task,
    color: Color,
    style: models::Style,
    is_selected: bool,
}

impl<'a> TaskLine<'a> {
    pub fn new(task: &'a Task, color: Color, style: models::Style, is_selected: bool) -> Self {
        Self {
            task,
            color,
            style,
            is_selected,
        }
    }
}

impl<'a> From<TaskLine<'a>> for Line<'a> {
    fn from(value: TaskLine) -> Self {
        let Task { title, .. } = value.task;

        let checkbox = if value.style == models::Style::Completed {
            "[x]"
        } else {
            "[ ]"
        };

        Line::styled(
            format!("{checkbox} {title}"),
            if value.is_selected {
                Style::default().reversed()
            } else {
                Style::default().fg(value.color)
            },
        )
    }
}
