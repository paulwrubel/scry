use crate::tui::action::Action;
use crate::tui::component::{RenderContext, State};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct ConfirmDelete {
    task_id: i64,
    task_title: String,
}

impl ConfirmDelete {
    pub fn new(task_id: i64, task_title: String) -> Self {
        Self {
            task_id,
            task_title,
        }
    }

    pub fn handle_event(&mut self, _state: &State, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => Some(Action::DismissPopup),
            KeyCode::Delete | KeyCode::Char('y') => Some(Action::DeleteTask(self.task_id)),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let content_area = ctx.render_popup_frame(
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Some(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Confirm Deletion?")
                    .title(Line::from("Esc").right_aligned().dim()),
            ),
        );

        let [_, prompt_area, buttons_area] = content_area.layout(&Layout::vertical([
            Constraint::Length(1), // padding
            Constraint::Fill(1),
            Constraint::Length(1),
        ]));

        ctx.with_area(prompt_area).render(
            Paragraph::new(Line::from(format!("Delete task: \"{}\"?", self.task_title)))
                .centered()
                .wrap(Wrap { trim: false }),
        );

        let (no_button, yes_button) = (
            Line::from(vec![Span::from("[n / esc]"), Span::from(" cancel").dim()]),
            Line::from(vec![Span::from("[y / del]"), Span::from(" delete").dim()]),
        );

        let [no_button_area, yes_button_area] = buttons_area.layout(
            &Layout::horizontal([
                Constraint::Length(
                    no_button
                        .width()
                        .try_into()
                        .expect("If this panics, i'll eat my hat"),
                ),
                Constraint::Length(
                    yes_button
                        .width()
                        .try_into()
                        .expect("If this panics, i'll eat my hat"),
                ),
            ])
            .flex(Flex::SpaceAround),
        );

        ctx.with_area(no_button_area).render(no_button);
        ctx.with_area(yes_button_area).render(yes_button);
    }
}
