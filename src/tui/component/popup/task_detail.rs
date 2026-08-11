use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

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
            Line::from(""),
        ];

        if let Some(ref desc) = task.description {
            lines.push(Line::from("  Description:"));
            lines.push(Line::from(format!("  {}", desc)));
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "  Press Esc or Enter to close.",
            Style::default().add_modifier(Modifier::DIM),
        )));

        let height = (lines.len() + 2) as u16;
        let width = 60u16;
        render_centered_popup(
            ctx.frame,
            lines,
            height.min(ctx.frame.area().height),
            width,
            "Task Detail",
        );
    }
}

fn render_centered_popup(
    frame: &mut Frame,
    lines: Vec<Line>,
    height: u16,
    width: u16,
    title: &str,
) -> Rect {
    let area = frame.area();
    let popup_area = centered_rect(width, height, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(title);

    // clear the area behind the popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(popup_area)[1];

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner)[1];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    popup_area
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
            Constraint::Length(percent_y),
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
            Constraint::Length(percent_x),
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
        ])
        .split(popup_layout[1])[1]
}
