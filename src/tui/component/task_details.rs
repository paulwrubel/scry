use crate::models::Task;
use crate::tui::component::RenderContext;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};

pub struct TaskDetails<'a> {
    task: &'a Task,
}

impl<'a> TaskDetails<'a> {
    pub fn new(task: &'a Task) -> Self {
        Self { task }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let status = ctx
            .state
            .statuses
            .iter()
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

        ctx.with_area(details_area)
            .render(Paragraph::new(details_text).wrap(Wrap { trim: false }));
    }
}
