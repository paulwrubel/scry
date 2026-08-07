use crate::tui::app::{App, PopupState, SettingsMode};
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
            PopupState::ProjectSettings {
                selected_row, mode, ..
            } => render_project_settings(frame, app, *selected_row, mode),
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

fn render_project_settings(frame: &mut Frame, app: &App, selected_row: usize, mode: &SettingsMode) {
    let mut lines: Vec<Line> = Vec::new();
    let states = &app.states;

    // PickingColor mode: side-by-side layout
    if let SettingsMode::PickingColor {
        selected_color_index,
        ..
    } = mode
    {
        let color_idx = *selected_color_index;

        // ── build left panel (compact settings view) ──
        let mut left: Vec<Line> = Vec::new();
        left.push(Line::from(Span::styled(
            "  Project",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        left.push(Line::from(Span::styled(
            format!("    {}", app.project.name),
            Style::default(),
        )));
        left.push(Line::from(""));
        left.push(Line::from(Span::styled(
            "  States",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for (i, state) in states.iter().enumerate() {
            let row_idx = i + 1;
            let is_selected = row_idx == selected_row;
            let marker = if is_selected { ">" } else { " " };
            let color_name = state.color.as_ref().map(|c| c.0.as_str()).unwrap_or("none");
            left.push(Line::from(format!(
                "  {} {}. {} [{}]",
                marker, row_idx, state.name, color_name
            )));
        }
        if states.is_empty() {
            left.push(Line::from("    (no states)"));
        }

        // ── build right panel (color picker) ──
        let mut right: Vec<Line> = Vec::new();
        right.push(Line::from(Span::styled(
            "  Color",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        right.push(Line::from(""));

        // "None" option (index 0)
        let is_none_selected = color_idx == 0;
        let marker = if is_none_selected { ">" } else { " " };
        let style = if is_none_selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        right.push(Line::from(Span::styled(
            format!("  {} None (default)", marker),
            style,
        )));

        // named colors
        for (i, (name, color)) in crate::models::STATE_COLORS.iter().enumerate() {
            let cidx = i + 1;
            let is_selected = cidx == color_idx;
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(*color)
            };
            right.push(Line::from(Span::styled(
                format!("  {} {}", marker, name),
                style,
            )));
        }

        // ── merge side-by-side ──
        let left_width = 34usize;
        let max_rows = left.len().max(right.len());
        let mut lines: Vec<Line> = Vec::new();

        for row in 0..max_rows {
            let left_text = if row < left.len() {
                let line_str = left[row]
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<Vec<&str>>()
                    .join("");
                format!("{:<width$}", line_str, width = left_width)
            } else {
                " ".repeat(left_width)
            };

            let right_spans = if row < right.len() {
                right[row].spans.clone()
            } else {
                vec![Span::raw("")]
            };

            let mut all_spans = vec![Span::raw(left_text), Span::raw(" ")];
            all_spans.extend(right_spans);
            lines.push(Line::from(all_spans));
        }

        let height = (lines.len() + 2) as u16;
        let width = 68u16;
        let title = format!(" Project Settings: {}", app.project.name);
        render_centered_popup(frame, lines, height.min(frame.area().height), width, &title);
        return;
    }

    // Project section
    lines.push(Line::from(Span::styled(
        "  Project",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    let row0_selected = selected_row == 0;

    match mode {
        SettingsMode::EditingName { input } if selected_row == 0 => {
            let text = format!("    Name:  {}", input.buffer);
            let style = if row0_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
            lines.push(Line::from(Span::styled(
                "           Enter confirm  Esc cancel",
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
        _ => {
            let text = format!("    Name:  {}", app.project.name);
            let style = if row0_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
            lines.push(Line::from(Span::styled(
                "           (r) rename",
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }

    lines.push(Line::from(""));

    // States section
    lines.push(Line::from(Span::styled(
        "  States",
        Style::default().add_modifier(Modifier::BOLD),
    )));

    if let SettingsMode::AddingState { input } = mode {
        let text = format!("    New:  {}", input.buffer);
        let style = if selected_row == 1 {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(text, style)));
        lines.push(Line::from(Span::styled(
            "          Enter confirm  Esc cancel",
            Style::default().add_modifier(Modifier::DIM),
        )));
    } else {
        for (i, state) in states.iter().enumerate() {
            let row_idx = i + 1;
            let is_selected = row_idx == selected_row;
            let is_editing = matches!(mode, SettingsMode::EditingName { .. }) && is_selected;

            if is_editing {
                if let SettingsMode::EditingName { input } = mode {
                    let text = format!("    {}. {}", i + 1, input.buffer);
                    let style = Style::default().add_modifier(Modifier::REVERSED);
                    lines.push(Line::from(Span::styled(text, style)));
                    lines.push(Line::from(Span::styled(
                        "        Enter confirm  Esc cancel",
                        Style::default().add_modifier(Modifier::DIM),
                    )));
                }
            } else {
                let marker = if is_selected { ">" } else { " " };
                let state_color_name = state.color.as_ref().map(|c| c.0.as_str());

                let color_text = match state_color_name {
                    Some(name) => format!("[{}]", name),
                    None => "[none]".to_string(),
                };

                let rat_color = state.color.clone().map(ratatui::style::Color::from);

                let color_style = if let Some(c) = rat_color {
                    Style::default().fg(c)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };

                let name_style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                let hints = if is_selected { "kj r c d" } else { "" };

                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled(
                    format!("  {} {}. ", marker, i + 1),
                    name_style,
                ));
                spans.push(Span::styled(format!("{:<20} ", state.name), name_style));
                spans.push(Span::styled(
                    format!("{:<10}", color_text),
                    if is_selected { name_style } else { color_style },
                ));
                if is_selected {
                    spans.push(Span::styled(
                        format!(" {}", hints),
                        Style::default().add_modifier(Modifier::DIM),
                    ));
                }

                lines.push(Line::from(spans));
            }
        }
    }

    if states.is_empty() && !matches!(mode, SettingsMode::AddingState { .. }) {
        lines.push(Line::from(Span::styled(
            "    (no states — press a to add)",
            Style::default().add_modifier(Modifier::DIM),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        Style::default().add_modifier(Modifier::DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  Esc close  k/j reorder  r rename  c color  d delete",
        Style::default().add_modifier(Modifier::DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  a add state",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let height = (lines.len() + 2) as u16;
    let width = 62u16;
    let title = format!(" Project Settings: {}", app.project.name);

    // set blinking cursor position when editing
    if let SettingsMode::EditingName { input } | SettingsMode::AddingState { input } = mode {
        let popup_area = centered_rect(width, height.min(frame.area().height), frame.area());

        let (input_line_idx, text_offset): (usize, u16) = if matches!(mode, SettingsMode::AddingState { .. }) {
            (5, 10) // "    New:  "
        } else if selected_row == 0 {
            (1, 11) // "    Name:  "
        } else {
            (4 + selected_row, 6 + (selected_row.to_string().len() as u16)) // "    {N}. "
        };

        let col = popup_area.x + 1 + text_offset + input.cursor_position as u16;
        let row = popup_area.y + 1 + input_line_idx as u16;
        frame.set_cursor_position((col, row));
    }

    let _ = render_centered_popup(frame, lines, height.min(frame.area().height), width, &title);
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
