use crate::models::Task;
use crate::tui::component::RenderContext;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub struct TaskDetails<'a> {
    task: &'a Task,
}

impl<'a> TaskDetails<'a> {
    pub fn new(task: &'a Task) -> Self {
        Self { task }
    }

    // pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
    //     match key.code {
    //         KeyCode::Esc | KeyCode::Enter => Some(Action::DismissPopup),
    //         _ => None,
    //     }
    // }

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

        let [details_area] = ctx.area.layout(
            &Layout::vertical([Constraint::Min(1)]).horizontal_margin(1), // .spacing(Spacing::Space(1)),
        );

        let mut details_lines = Vec::new();

        // title
        details_lines.push(Line::from(vec![
            Span::from(self.task.title.clone()).bold(),
            Span::from(format!(" #{}", self.task.id)).dim().italic(),
        ]));

        // description
        if let Some(desc) = self.task.description.clone() {
            details_lines.push(Line::from(desc));
        };

        // info
        details_lines.append(&mut vec![
            Line::from(vec![
                Span::from("Status:      "),
                Span::from(status.name.clone()),
            ]),
            Line::from(vec![Span::from("Created at:  "), Span::from(created)]),
        ]);

        ctx.with_area(details_area)
            .render(Paragraph::new(details_lines));
    }
}
