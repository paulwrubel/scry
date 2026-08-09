use ratatui::Frame;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

use crate::tui::action::Action;
use crate::tui::component::AppContext;
use crate::tui::component::Component;

pub struct StatusBar {
    // ── internal ──
    pub message: String,
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            message: String::new(),
        }
    }

    pub fn set_message(&mut self, msg: String) {
        self.message = msg;
    }
}

impl Component for StatusBar {
    fn handle_event(&mut self, _ctx: &AppContext, _key: KeyEvent) -> Option<Action> {
        // passive component, does not handle events
        None
    }

    fn render(&self, _ctx: &AppContext, frame: &mut Frame, area: Rect) {
        let (text, style) = if self.message.is_empty() {
            (
                String::from(
                    "[a]dd task | [m]ove task | [d]elete task | project [s]ettings | [Enter]: task details | [q]uit",
                ),
                Style::default(),
            )
        } else {
            (
                self.message.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )
        };

        frame.render_widget(Paragraph::new(text).style(style), area);
    }
}
