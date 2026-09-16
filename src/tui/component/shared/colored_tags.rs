use crate::{color::tag_rgb, models::Tags};
use ratatui::{
    style::{Color, Stylize},
    text::Span,
};

#[derive(Debug, Clone)]
pub struct ColoredTags(Tags);

impl ColoredTags {
    pub fn new(tags: Tags) -> Self {
        Self(tags)
    }

    pub fn spans(&self) -> Vec<Span<'static>> {
        itertools::intersperse(
            self.0.iter().map(|tag| {
                let (r, g, b) = tag_rgb(tag);
                tag.clone().italic().fg(Color::Rgb(r, g, b))
            }),
            Span::from(" "),
        )
        .collect()
    }
}
