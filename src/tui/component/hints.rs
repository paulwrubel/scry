use crate::tui::component::RenderContext;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub struct Hints {
    pub message: String,
}

impl Hints {
    pub fn new() -> Self {
        Hints {
            message: String::new(),
        }
    }

    pub fn set_message(&mut self, msg: String) {
        self.message = msg;
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let hint_actions = [
            ["[a]", "dd task"],
            ["[m]", "ove task"],
            ["[d]", "elete task"],
            ["[Enter]", " task details"],
        ];

        let spans = if self.message.is_empty() {
            hint_actions
                .into_iter()
                .flat_map(|[key, desc]| {
                    [
                        Span::styled(" • ", Style::default().dim()),
                        Span::styled(key, Style::default()),
                        Span::styled(desc, Style::default().dim()),
                    ]
                })
                .skip(1)
                .collect::<Vec<Span>>()
        } else {
            vec![Span::styled(
                self.message.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )]
        };

        ctx.render(Line::from(spans));
    }
}
