use crate::tui::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if app.error_message.is_empty() {
        (
            String::from(
                "a: focus input | m: move | c: color | d: delete | Enter: detail | q: quit",
            ),
            Style::default(),
        )
    } else {
        (
            app.error_message.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}
