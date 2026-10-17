use crate::state::DATETIME_FORMAT_STR;
use crate::state::{ProjectState, TaskWithNotes};
use crate::tui::component::RenderContext;
use crate::tui::component::shared::ColoredTags;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};
use std::iter;

#[derive(Debug, Clone)]
pub struct TaskDetails {
    task: Option<TaskWithNotes>,
    scroll_offset: u16,
}

impl TaskDetails {
    pub fn new(task: Option<TaskWithNotes>) -> Self {
        Self {
            task,
            scroll_offset: 0,
        }
    }

    pub fn set_task(&mut self, task: Option<TaskWithNotes>) {
        let current_task_id = self.task.as_ref().map(|task| task.id);
        let new_task_id = task.as_ref().map(|task| task.id);
        if new_task_id != current_task_id {
            self.scroll_offset = 0;
        }

        self.task = task;
    }

    pub fn handle_event(&mut self, _state: &ProjectState, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (modifiers, KeyCode::Left) => {
                let scroll_amount = if modifiers == KeyModifiers::SHIFT {
                    3
                } else {
                    1
                };
                self.scroll_offset = self.scroll_offset.saturating_sub(scroll_amount);
            }
            (modifiers, KeyCode::Right) => {
                let scroll_amount = if modifiers == KeyModifiers::SHIFT {
                    3
                } else {
                    1
                };
                self.scroll_offset = self.scroll_offset.saturating_add(scroll_amount);
            }
            _ => {}
        };
    }

    pub fn render(&mut self, ctx: &mut RenderContext) {
        let Some(task) = &self.task else {
            return;
        };

        let status = ctx
            .state
            .get_status_by_id(task.status_id)
            .expect("task should have a status");

        let created = task
            .created_at
            .with_timezone(&chrono::Local)
            .format(DATETIME_FORMAT_STR)
            .to_string();

        let [details_area] = ctx
            .area
            .layout(&Layout::vertical([Constraint::Min(1)]).horizontal_margin(1));

        let mut details_text = Text::default();

        // title
        details_text.push_line(Line::from(vec![
            Span::from(task.title.clone()).bold(),
            Span::from(format!(" #{}", task.id)).dim().italic(),
        ]));
        details_text.push_line(Line::default());

        // description
        if let Some(desc) = &task.description {
            for line in desc.lines() {
                details_text.push_line(Line::from(vec![
                    Span::raw("    "), // indent
                    Span::raw(line),
                ]));
            }
        };
        details_text.push_line(Line::default());

        // info
        details_text.push_line(Line::from(vec![
            Span::from("Priority:    "),
            Span::from(task.priority),
        ]));
        details_text.push_line(Line::from(vec![
            Span::from("Status:      "),
            Span::from(status.name.clone()),
        ]));
        details_text.push_line(Line::from(
            iter::once(Span::from("Tags:        "))
                .chain(ColoredTags::new(task.tags.clone()).spans())
                .collect::<Vec<Span>>(),
        ));
        details_text.push_line(Line::from(vec![
            Span::from("Created at:  "),
            Span::from(created),
        ]));
        details_text.push_line(Line::default());

        // notes
        for note in &task.notes {
            let date_span = Span::from(
                note.created_at
                    .with_timezone(&chrono::Local)
                    .format(DATETIME_FORMAT_STR)
                    .to_string(),
            );
            let second_line_width = 1;
            let first_line_width = details_area
                .width
                .saturating_sub(date_span.width() as u16)
                .saturating_sub(2) // padding
                .saturating_sub(second_line_width);
            details_text.push_line(
                Line::from(vec![
                    Span::from("─".repeat(first_line_width.into())),
                    Span::from(" "),
                    date_span,
                    Span::from(" "),
                    Span::from("─".repeat(second_line_width.into())),
                ])
                .dim(),
            );

            details_text += Text::from(note.contents.clone());
            details_text.push_line(Line::default());
        }

        let paragraph = Paragraph::new(details_text).wrap(Wrap { trim: false });

        // line_count counts wrapped display rows, unlike Text::height()'s logical lines
        self.scroll_offset = self.scroll_offset.min(Self::max_scroll_offset(
            &paragraph,
            details_area.width,
            details_area.height,
        ));

        ctx.with_area(details_area)
            .render(paragraph.scroll((self.scroll_offset, 0)));
    }

    fn max_scroll_offset(paragraph: &Paragraph<'_>, width: u16, area_height: u16) -> u16 {
        (paragraph.line_count(width) as u16).saturating_sub(area_height)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskDetails;
    use crate::state::TaskWithNotes;
    use chrono::Utc;
    use ratatui::widgets::{Paragraph, Wrap};
    use scry_core::models::{Priority, Tags};

    fn task(id: i64) -> TaskWithNotes {
        TaskWithNotes {
            id,
            project_id: 1,
            title: format!("task {id}"),
            description: None,
            priority: Priority::default(),
            status_id: 1,
            position: id as i32,
            tags: Tags::default(),
            created_at: Utc::now(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn long_wrapped_text_reports_scrollable_height() {
        let paragraph = Paragraph::new("word ".repeat(200)).wrap(Wrap { trim: false });
        assert!(TaskDetails::max_scroll_offset(&paragraph, 20, 10) > 0);
    }

    #[test]
    fn short_text_does_not_scroll() {
        let paragraph = Paragraph::new("short").wrap(Wrap { trim: false });
        assert_eq!(TaskDetails::max_scroll_offset(&paragraph, 20, 10), 0);
    }

    #[test]
    fn set_task_resets_scroll_only_when_task_changes() {
        let mut details = TaskDetails::new(Some(task(1)));
        details.scroll_offset = 5;

        details.set_task(Some(task(1)));
        assert_eq!(details.scroll_offset, 5);

        details.set_task(Some(task(2)));
        assert_eq!(details.scroll_offset, 0);
    }
}
