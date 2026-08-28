use crate::tui::component::{RenderContext, TaskWithNotes};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};

pub struct TaskDetails<'a> {
    task: &'a TaskWithNotes,
}

impl<'a> TaskDetails<'a> {
    pub fn new(task: &'a TaskWithNotes) -> Self {
        Self { task }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let status = ctx
            .state
            .statuses()
            .find(|s| s.id == self.task.status_id)
            .expect("no status for task!");

        let created = self
            .task
            .created_at
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %I:%M %p")
            .to_string();

        let [details_area] = ctx
            .area
            .layout(&Layout::vertical([Constraint::Min(1)]).horizontal_margin(1));

        let mut details_text = Text::default();

        // title
        details_text.push_line(Line::from(vec![
            Span::from(self.task.title.clone()).bold(),
            Span::from(format!(" #{}", self.task.id)).dim().italic(),
        ]));
        details_text.push_line(Line::default());

        // description
        if let Some(desc) = &self.task.description {
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
            Span::from("Status:      "),
            Span::from(status.name.clone()),
        ]));
        details_text.push_line(Line::from(vec![
            Span::from("Created at:  "),
            Span::from(created),
        ]));
        details_text.push_line(Line::default());

        // notes
        for note in &self.task.notes {
            let date_span = Span::from(
                note.created_at
                    .with_timezone(&chrono::Local)
                    .format("%b %d, %Y at %I:%M %P")
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

        ctx.with_area(details_area)
            .render(Paragraph::new(details_text).wrap(Wrap { trim: false }));
    }
}
