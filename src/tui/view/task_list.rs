use crate::{models::Task, tui::app::App};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let selected_id = app.selected_task().map(|t| t.id);

    for state in &app.states {
        let state_tasks: Vec<_> = app
            .tasks
            .iter()
            .filter(|t| t.state_name == state.name)
            .collect();

        lines.push(render_state_header(&state.name, state_tasks.len()));

        for task in state_tasks {
            let selected = Some(task.id) == selected_id;
            lines.push(render_task_row(task, selected));
        }
    }

    let paragraph = Paragraph::new(lines).scroll((app.scroll_offset, 0));

    frame.render_widget(paragraph, area);
}

fn render_state_header(state_name: &str, task_count: usize) -> Line<'_> {
    Line::from(Span::styled(
        format!("{} ({}):", state_name, task_count),
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

fn render_task_row(task: &Task, selected: bool) -> Line<'_> {
    let Task {
        id,
        title,
        completed_at,
        ..
    } = task;

    let checkbox = if completed_at.is_some() { "[x]" } else { "[ ]" };
    let row_text = format!(" {id:>3} {checkbox} | {title}");

    let style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    Line::styled(row_text, style)
}
