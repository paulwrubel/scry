use crate::tui::{
    Action,
    component::{ProjectState, RenderContext},
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    widgets::Paragraph,
};
use ratatui_textarea::TextArea;

pub struct FilterInput {
    pub is_focused: bool,

    textarea: TextArea<'static>,
}

impl FilterInput {
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,

            textarea: TextArea::default(),
        }
    }

    pub fn reset(&mut self) {
        self.textarea = TextArea::default();
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.is_focused = false;
    }

    pub fn current_filter(&self) -> String {
        self.textarea.lines().first().cloned().unwrap_or_default()
    }

    pub fn handle_event(&mut self, _state: &ProjectState, key: KeyEvent) -> Vec<Action> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.reset();
                vec![Action::CloseFilterInput]
            }
            (_, KeyCode::Enter) => {
                vec![Action::CloseFilterInput]
            }
            _ => {
                self.textarea.input(key);
                vec![]
            }
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        // nothing to filter, nothing to show
        if !self.is_focused && self.current_filter().is_empty() {
            return;
        };

        let mut style = Style::default();
        if !self.is_focused {
            style = style.dim()
        };

        let [prefix_area, input_area] = ctx.area.layout(&Layout::horizontal([
            Constraint::Length(1),
            Constraint::Min(0),
        ]));

        let mut textarea = self.textarea.clone();
        textarea.set_style(style);
        textarea.set_cursor_line_style(Style::default());

        ctx.with_area(prefix_area).render(Paragraph::new("/").dim());
        ctx.with_area(input_area).render(&textarea);
    }
}
