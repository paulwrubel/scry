use crate::{
    models::{Color as ScryColor, Style as ScryStyle},
    tui::component::{TaskWithNotes, shared::truncate_string_to_width},
};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct TaskLine {
    task: TaskWithNotes,
    color: Option<ScryColor>,
    style: ScryStyle,
    is_selected: bool,
    area_width: u16,
}

impl TaskLine {
    pub fn new(
        task: TaskWithNotes,
        color: Option<ScryColor>,
        style: ScryStyle,
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
        let prefix = vec![
            Span::from(if matches!(value.style, ScryStyle::Completed) {
                "[x]"
            } else {
                "[ ]"
            }),
            Span::from(" "),
        ];

        let prefix_length: usize = prefix.iter().map(|s| s.width()).sum();
        let task_span = Span::styled(
            truncate_string_to_width(
                value.task.title.clone(),
                usize::from(value.area_width).saturating_sub(prefix_length),
            ),
            if value.is_selected {
                Style::default().reversed()
            } else {
                Style::default().fg(value.color.map_or(Color::default(), |c| c.into()))
            },
        );

        Line::from(prefix.into_iter().chain([task_span]).collect::<Vec<_>>())
    }
}
