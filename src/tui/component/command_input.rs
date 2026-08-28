use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::Style,
    widgets::Paragraph,
};
use ratatui_textarea::TextArea;

use crate::tui::{
    Action, command,
    component::{ProjectState, RenderContext},
};

pub struct CommandInput {
    pub is_focused: bool,

    textarea: TextArea<'static>,
}

impl CommandInput {
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

    pub fn handle_event(&mut self, state: &ProjectState, key: KeyEvent) -> Vec<Action> {
        // Enter is handled below (submit); never let it insert a newline
        match (key.modifiers, key.code) {
            (_, KeyCode::Enter) | (KeyModifiers::CONTROL, KeyCode::Char('m')) => {}
            _ => {
                self.textarea.input(key);
            }
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => vec![Action::CloseCommandInput],
            (_, KeyCode::Enter) => {
                let mut actions = vec![];

                let mut command_actions = self.process_command_text(state);
                if !command_actions.is_empty() {
                    self.reset();
                    actions.append(&mut command_actions);
                }
                actions.push(Action::CloseCommandInput);

                actions
            }
            _ => vec![],
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let style = if self.is_focused {
            Style::default()
        } else {
            Style::default().dim()
        };

        let [prefix_area, input_area] =
            Layout::horizontal([Constraint::Length(1), Constraint::Min(0)]).areas(ctx.area);
        ctx.with_area(prefix_area)
            .render(Paragraph::new("/").style(style));

        let mut textarea = self.textarea.clone();
        textarea.set_style(style);
        textarea.set_cursor_line_style(Style::default());

        ctx.with_area(input_area).render(&textarea);
    }

    fn process_command_text(&self, state: &ProjectState) -> Vec<Action> {
        let text = self.textarea.lines().join("\n");

        command::parse_command(state, &text)
    }
}
