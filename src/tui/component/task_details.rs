use crate::tui::component::RenderContext;
use crate::tui::component::shared::{ColoredTags, DATETIME_FORMAT_STR};
use crate::tui::state::{ProjectState, TaskWithNotes};
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

        // clamp the scroll amount
        let text_height = details_text.height() as u16;
        let area_height = ctx.area.height;
        self.scroll_offset = self
            .scroll_offset
            .clamp(0, text_height.saturating_sub(area_height));

        ctx.with_area(details_area).render(
            Paragraph::new(details_text)
                .scroll((self.scroll_offset, 0))
                .wrap(Wrap { trim: false }),
        );
    }
}
