use crate::tui::app::{App, PopupState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame, app: &App) {
    if let Some(ref popup) = app.popup {
        match popup {
            PopupState::TaskDetail { task_id } => render_task_detail(frame, app, *task_id),
            PopupState::StatePicker {
                task_id: _,
                selected_state_index,
            } => render_state_picker(frame, app, *selected_state_index),
            PopupState::ConfirmDelete {
                task_id: _,
                task_title,
                confirm,
            } => render_confirm_delete(frame, task_title, *confirm),
        }
    }
}

fn render_task_detail(frame: &mut Frame, app: &App, task_id: i64) {
    let task = app.tasks.iter().find(|t| t.id == task_id);
    if let Some(task) = task {
        let created = task.created_at.format("%Y-%m-%d %I:%M %p").to_string();
        let state_name = app
            .states
            .iter()
            .find(|s| s.id == task.state_id)
            .map(|s| s.name.as_str())
            .unwrap_or("unknown");

        let lines = vec![
            Line::from(Span::styled(
                format!("Task {}", task.id),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("  Title:     {}", task.title)),
            Line::from(format!("  State:     {}", state_name)),
            Line::from(format!("  Project:   {}", app.project.name)),
            Line::from(format!("  Created:   {}", created)),
            Line::from(""),
        ];

        let mut all_lines = lines.clone();
        if let Some(ref desc) = task.description {
            all_lines.push(Line::from("  Description:"));
            all_lines.push(Line::from(format!("  {}", desc)));
            all_lines.push(Line::from(""));
        }
        all_lines.push(Line::from(Span::styled(
            "  Press Esc or Enter to close.",
            Style::default().add_modifier(Modifier::DIM),
        )));

        let height = (all_lines.len() + 2) as u16;
        let width = 60u16;
        render_centered_popup(
            frame,
            all_lines,
            height.min(frame.area().height),
            width,
            "Task Detail",
        );
    }
}

fn render_state_picker(frame: &mut Frame, app: &App, selected_state_index: usize) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, state) in app.states.iter().enumerate() {
        let marker = if i == selected_state_index { ">" } else { " " };
        let is_current = app
            .selected_task()
            .map(|t| t.state_id == state.id)
            .unwrap_or(false);
        let suffix = if is_current { " (current)" } else { "" };
        let text = format!("  {} {}{}", marker, state.name, suffix);

        let style = if i == selected_state_index {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(text, style)));
    }
    if lines.is_empty() {
        lines.push(Line::from("  (no states)"));
    }

    let height = (lines.len() + 2) as u16;
    let width = 40u16;
    render_centered_popup(
        frame,
        lines,
        height.min(frame.area().height),
        width,
        "Move task to...",
    );
}

fn render_confirm_delete(frame: &mut Frame, task_title: &str, confirm: bool) {
    let lines = [
        Line::from(format!("  Delete task \"{}\"?", task_title)),
        Line::from(""),
    ];

    let yes_style = if confirm {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let no_style = if !confirm {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    let button_line = Line::from(vec![
        Span::styled("        ", Style::default()),
        Span::styled("[n] No", no_style),
        Span::styled("   ", Style::default()),
        Span::styled("[y] Yes", yes_style),
        Span::styled("        ", Style::default()),
    ]);

    let all_lines = vec![lines[0].clone(), lines[1].clone(), button_line];

    let height = 5u16;
    let width = 44u16;
    render_centered_popup(
        frame,
        all_lines,
        height.min(frame.area().height),
        width,
        "Confirm",
    );
}

fn render_centered_popup(
    frame: &mut Frame,
    lines: Vec<Line>,
    height: u16,
    width: u16,
    title: &str,
) {
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
