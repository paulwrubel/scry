use crate::tui::component::RenderContext;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

pub struct HintBar {
    pub message: String,
}

impl HintBar {
    pub fn new() -> Self {
        HintBar {
            message: String::new(),
        }
    }

    pub fn set_message(&mut self, msg: String) {
        self.message = msg;
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let hint_actions = [
            "[a]dd task",
            "[m]ove task",
            "[d]elete task",
            "[Enter]: task details",
            "[q]uit",
        ];
        let (text, style) = if self.message.is_empty() {
            (hint_actions.join(" | "), Style::default())
        } else {
            (
                self.message.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )
        };

        ctx.render_widget(Paragraph::new(text).style(style));
    }
}
