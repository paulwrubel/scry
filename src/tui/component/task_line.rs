use std::iter;

use crate::{
    models::{Color as ScryColor, StatusStyle},
    tui::{component::shared::truncate_string_to_width, state::TaskWithNotes},
};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct TaskLine {
    task: TaskWithNotes,
    color: Option<ScryColor>,
    style: StatusStyle,
    is_selected: bool,
    area_width: u16,
}

impl TaskLine {
    pub fn new(
        task: TaskWithNotes,
        color: Option<ScryColor>,
        style: StatusStyle,
        is_selected: bool,
        area_width: u16,
    ) -> Self {
        Self {
            task,
            color,
            style,
            is_selected,
            area_width,
        }
    }
}

impl From<TaskLine> for Line<'_> {
    fn from(value: TaskLine) -> Self {
        let prefix = Span::from(match value.style {
            StatusStyle::None | StatusStyle::Strikethrough => "",
            StatusStyle::Unchecked => "[ ] ",
            StatusStyle::Checked => "[x] ",
        });

        let mut text_style =
            Style::default().fg(value.color.map_or(Color::default(), |c| c.into()));
        if value.is_selected {
            text_style = text_style.reversed()
        };
        if value.style == StatusStyle::Strikethrough {
            text_style = text_style.crossed_out()
        }

        let prefix_length: usize = prefix.width();
        let task_span = Span::styled(
            truncate_string_to_width(
                value.task.title.clone(),
                usize::from(value.area_width).saturating_sub(prefix_length),
            ),
            text_style,
        );

        Line::from(iter::once(prefix).chain([task_span]).collect::<Vec<_>>())
    }
}
