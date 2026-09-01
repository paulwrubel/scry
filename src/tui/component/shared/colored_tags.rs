use crate::models::Tags;
use ratatui::{
    style::{Color, Stylize},
    text::Span,
};
use std::str::FromStr;
use twox_hash::XxHash32;

// I just mashed my keyboard
const HASH_SEED: u32 = 1870435234;
const TAG_COLORS: [&str; 14] = [
    "#FFD6E0", "#FFC9B9", "#FFE0BD", "#FBF0A8", "#DCEDC1", "#B8E6D2", "#AEE3E8", "#BBDCF7",
    "#C6CBF5", "#D9C7F0", "#EFC7E8", "#FAF7F2", "#D6D0C8", "#5C5470",
];

#[derive(Debug, Clone)]
pub struct ColoredTags(Tags);

impl ColoredTags {
    pub fn new(tags: Tags) -> Self {
        Self(tags)
    }

    pub fn spans(&self) -> Vec<Span<'static>> {
        itertools::intersperse(
            self.0.iter().map(|tag| {
                let hash = XxHash32::oneshot(HASH_SEED, tag.as_bytes());
                let index = hash as usize % TAG_COLORS.len();

                tag.clone()
                    .italic()
                    .fg(Color::from_str(TAG_COLORS[index]).expect("probably won't fail i think"))
            }),
            Span::from(" "),
        )
        .collect()
    }
}
