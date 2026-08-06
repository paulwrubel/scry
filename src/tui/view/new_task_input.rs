use crate::tui::app::InputState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(
    frame: &mut Frame,
    state: &InputState,
    area: Rect,
    selected: bool,
) -> Option<(u16, u16)> {
    use ratatui::layout::{Constraint, Direction, Layout};

    // build a block around the input area when selected or focused
    let block = if state.focused {
        Block::default().borders(Borders::ALL).title("New Task")
    } else if selected {
        Block::default().borders(Borders::ALL)
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().add_modifier(Modifier::DIM))
    };

    // split the area vertically to center the text (input area is 3 rows)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(block.inner(area));

    let text_area = rows[1];

    let lines = if state.focused {
        if state.buffer.is_empty() {
            vec![Line::from(Span::raw(" "))]
        } else {
            vec![Line::from(Span::raw(&state.buffer))]
        }
    } else if selected {
        vec![Line::from(Span::styled(
            "Add a task...",
            Style::default().add_modifier(Modifier::REVERSED),
        ))]
    } else {
        vec![Line::from(Span::styled(
            "Add a task...",
            Style::default().add_modifier(Modifier::DIM),
        ))]
    };

    frame.render_widget(block, area);
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, text_area);

    if state.focused {
        let col = if state.buffer.is_empty() {
            text_area.x
        } else {
            text_area.x + state.cursor_position.min(state.buffer.len()) as u16
        };
        Some((col, text_area.y))
    } else {
        None
    }
}
