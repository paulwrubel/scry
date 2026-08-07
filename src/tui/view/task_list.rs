use crate::{models::Task, tui::app::App};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let selected_id = app.selected_task().map(|t| t.id);

    for (i, state) in app.states.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }

        let state_tasks: Vec<_> = app
            .tasks
            .iter()
            .filter(|t| t.state_id == state.id)
            .collect();

        lines.push(render_state_header(&state.name, state_tasks.len()));

        for task in state_tasks {
            let selected = Some(task.id) == selected_id;
            lines.push(render_task_row(
                task,
                selected,
                state.is_completed,
                state.color.clone().map(Into::into),
            ));
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

fn render_task_row<'a>(
    task: &'a Task,
    selected: bool,
    is_completed: bool,
    state_color: Option<Color>,
) -> Line<'a> {
    let Task { id, title, .. } = task;

    let checkbox = if is_completed { "[x]" } else { "[ ]" };
    let row_text = format!(" {id:>3} {checkbox} {title}");

    let mut style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    if !selected && let Some(color) = state_color {
        style = style.fg(color);
    }

    Line::styled(row_text, style)
}
