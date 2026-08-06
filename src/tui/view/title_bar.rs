use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
};

pub fn render(frame: &mut Frame, project_name: &str, area: Rect) {
    let title = format!("scry | project: {project_name}");
    let paragraph = Paragraph::new(title)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
