use crate::{
    state::StatusWithTasks,
    tui::component::{TaskLine, shared::truncate_string_to_width},
};
use scry_core::models::{StatusStyle, TaskId};
use ratatui::{
    style::{Color, Stylize},
    text::{Line, Span, Text},
};

pub struct TaskStatusList<'a> {
    status_with_tasks: &'a StatusWithTasks,
    selected_task_id: Option<TaskId>,
    show_priority: bool,
    area_width: u16,
}

impl<'a> TaskStatusList<'a> {
    pub fn new(
        status_with_tasks: &'a StatusWithTasks,
        selected_task_id: Option<TaskId>,
        show_priority: bool,
        area_width: u16,
    ) -> Self {
        Self {
            status_with_tasks,
            selected_task_id,
            show_priority,
            area_width,
        }
    }
}

impl<'a> From<TaskStatusList<'a>> for Text<'a> {
    fn from(value: TaskStatusList<'a>) -> Self {
        let status: &scry_core::models::Status = &value.status_with_tasks.status;
        let is_entry = value.status_with_tasks.is_entry;
        let status_name = &status.name;
        let is_hidden = status.style == StatusStyle::Hidden;
        let hidden_suffix = if is_hidden { " <hidden>" } else { "" };
        let task_count = value.status_with_tasks.tasks_with_notes.len()
            + value.status_with_tasks.hidden_task_count;

        let status_color = status.color.map_or(Color::default(), |c| c.into());

        let mut text = Text::default();

        let task_count_str = if is_entry {
            format!("* [{task_count}]")
        } else {
            format!("[{task_count}]")
        };
        let status_name_str = truncate_string_to_width(
            status_name.clone(),
            value
                .area_width
                .saturating_sub(task_count_str.chars().count() as u16)
                .saturating_sub(hidden_suffix.chars().count() as u16)
                .saturating_sub(1)
                .into(),
        );
        text.push_line(
            Line::from(vec![
                Span::from(status_name_str.clone()).italic(),
                Span::from(hidden_suffix),
                Span::from(
                    " ".repeat(
                        value
                            .area_width
                            .saturating_sub(
                                // status name is truncated above, there's no risk here
                                status_name_str.chars().count() as u16
                                // and for task count, this is essentially statically bounded
                                    + task_count_str.chars().count() as u16
                                    + hidden_suffix.chars().count() as u16,
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
                value.show_priority,
                value.area_width,
            )))
        }) {
            text += task_text;
        }

        text
    }
}

#[cfg(test)]
mod tests {
    use super::TaskStatusList;
    use crate::state::StatusWithTasks;
    use scry_core::models::{Status, StatusStyle};
    use ratatui::text::{Line, Text};

    fn status_with_tasks(style: StatusStyle, hidden_task_count: usize) -> StatusWithTasks {
        StatusWithTasks {
            status: Status {
                id: 1,
                project_id: 1,
                name: "Test Status".to_string(),
                position: 0,
                color: None,
                style,
            },
            is_entry: false,
            tasks_with_notes: Vec::new(),
            hidden_task_count,
        }
    }

    fn line_text(line: &Line<'_>) -> String {
        line.to_string()
    }

    #[test]
    fn hidden_status_header_shows_notice_and_real_count() {
        let swt = status_with_tasks(StatusStyle::Hidden, 3);
        let text = Text::from(TaskStatusList::new(&swt, None, false, 40));

        assert_eq!(text.lines.len(), 2);
        let header = line_text(&text.lines[0]);
        assert!(header.contains("<hidden>"), "header was {header:?}");
        assert!(header.contains("[3]"), "header was {header:?}");
    }

    #[test]
    fn visible_status_header_has_no_notice() {
        let swt = status_with_tasks(StatusStyle::None, 0);
        let text = Text::from(TaskStatusList::new(&swt, None, false, 40));

        let header = line_text(&text.lines[0]);
        assert!(header.contains("[0]"), "header was {header:?}");
        assert!(!header.contains("<hidden>"), "header was {header:?}");
    }
}
