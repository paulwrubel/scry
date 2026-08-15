use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct TaskDetail {
    task_id: i64,
}

impl TaskDetail {
    pub fn new(task_id: i64) -> Self {
        TaskDetail { task_id }
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => Some(Action::DismissPopup),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let Some(task) = ctx.state.tasks.iter().find(|t| t.id == self.task_id) else {
            return;
        };
        let status_name = ctx
            .state
            .statuses
            .iter()
            .find(|s| s.id == task.status_id)
            .map(|s| s.name.as_str())
            .unwrap_or("unknown");

        let created = task.created_at.format("%Y-%m-%d %I:%M %p").to_string();

        let mut lines = vec![
            Line::from(Span::styled(
                format!("Task {}", task.id),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("  Title:     {}", task.title)),
            Line::from(format!("  Status:    {}", status_name)),
            Line::from(format!("  Project:   {}", ctx.state.project.name)),
            Line::from(format!("  Created:   {}", created)),
        ];

        if let Some(ref desc) = task.description {
            lines.push(Line::from("  Description:"));
            lines.push(Line::from(format!("  {}", desc)));
        }
        lines.push(Line::from(Span::styled(
            "  Press Esc or Enter to close.",
            Style::default().add_modifier(Modifier::DIM),
        )));

        // let height = (lines.len() + 2) as u16;
        // let width = 60u16;

        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            Some(Block::default().borders(Borders::ALL).title("Task Detail")),
        );

        ctx.frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            content_area,
        );
    }
}
