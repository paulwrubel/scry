use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct ErrorInfo {
    error_text: String,
}

impl ErrorInfo {
    pub fn new(error_text: String) -> Self {
        Self { error_text }
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            // press basically anything to dismiss!
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(_) => Some(Action::DismissPopup),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Some(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().red())
                    .title(Line::from("Error!").centered())
                    .title(Line::from("Esc").right_aligned().dim()),
            ),
        );

        let [error_text_area, dismiss_prompt_area] = content_area.layout(
            &Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).vertical_margin(1),
        );

        ctx.with_area(error_text_area).render(
            Paragraph::new(Text::from(format!(
                "An error has occurred:\n{}",
                self.error_text
            )))
            .centered()
            .wrap(Wrap { trim: false }),
        );

        let dismiss_prompt =
            Paragraph::new(Line::from(vec![Span::from("[press any key to dismiss]")]))
                .centered()
                .wrap(Wrap { trim: false });

        ctx.with_area(dismiss_prompt_area).render(dismiss_prompt);
    }
}
