use crate::{
    models::{Color as ScryColor, StatusStyle},
    tui::{
        component::shared::{ColoredTags, truncate_string_to_width},
        state::TaskWithNotes,
    },
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
        let total_width = usize::from(value.area_width);

        let prefix = vec![Span::from(match value.style {
            StatusStyle::None | StatusStyle::Strikethrough => "",
            StatusStyle::Unchecked => "[ ] ",
            StatusStyle::Checked => "[x] ",
        })];
        let suffix = ColoredTags::new(value.task.tags).spans();

        let mut text_style =
            Style::default().fg(value.color.map_or(Color::default(), |c| c.into()));
        if value.is_selected {
            text_style = text_style.reversed()
        };
        if value.style == StatusStyle::Strikethrough {
            text_style = text_style.crossed_out()
        }

        let prefix_length: usize = prefix.iter().map(|s| s.width()).sum();
        let suffix_length: usize = suffix.iter().map(|s| s.width()).sum();
        let task_span = Span::styled(
            truncate_string_to_width(
                value.task.title.clone(),
                total_width
                    .saturating_sub(prefix_length)
                    .saturating_sub(suffix_length),
            ),
            text_style,
        );

        let used_width = prefix_length + task_span.width() + suffix_length;
        let spacing_span = Span::from(if used_width < total_width {
            " ".repeat(total_width.saturating_sub(used_width))
        } else {
            "".to_string()
        });

        Line::from(
            prefix
                .into_iter()
                .chain([task_span, spacing_span])
                .chain(suffix)
                .collect::<Vec<_>>(),
        )
    }
}
