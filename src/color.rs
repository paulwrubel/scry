use anstyle::{AnsiColor, Color as AnsiStyleColor, RgbColor, Style};
use clap::ValueEnum;
use twox_hash::XxHash32;

use crate::models::{Color, Priority};

/// When to colorize CLI output.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorChoice {
    /// Colorize only when stdout is a terminal and `NO_COLOR` is unset
    Auto,
    /// Always colorize
    Always,
    /// Never colorize
    Never,
}

impl ColorChoice {
    /// Install the choice globally so `anstream` emits or strips ANSI codes.
    pub fn apply(self) {
        match self {
            ColorChoice::Auto => anstream::ColorChoice::Auto,
            ColorChoice::Always => anstream::ColorChoice::Always,
            ColorChoice::Never => anstream::ColorChoice::Never,
        }
        .write_global();
    }
}

/// Render `text` with `style`. `anstream` strips the escape codes when color is
/// disabled, so callers can format unconditionally.
fn paint(style: Style, text: &str) -> String {
    format!("{style}{text}{style:#}")
}

/// A status name or header, colored by the status's configured color.
pub fn status(color: Option<Color>, text: &str) -> String {
    match color {
        Some(color) => paint(Style::new().fg_color(Some(AnsiColor::from(color).into())), text),
        None => text.to_string(),
    }
}

/// The TUI's short priority label (`pN`).
pub fn priority(priority: Priority) -> String {
    paint(
        Style::new().fg_color(Some(AnsiColor::from(priority.color()).into())),
        &format!("p{}", i64::from(priority)),
    )
}

/// The TUI's long priority label (`pN - Name`).
pub fn priority_long(priority: Priority) -> String {
    paint(
        Style::new().fg_color(Some(AnsiColor::from(priority.color()).into())),
        &format!("p{} - {}", i64::from(priority), priority),
    )
}

/// A tag, colored with the TUI's deterministic hash-based palette.
pub fn tag(tag: &str) -> String {
    let (r, g, b) = tag_rgb(tag);
    let style = Style::new().fg_color(Some(AnsiStyleColor::Rgb(RgbColor(r, g, b))));
    paint(style, tag)
}

// The TUI tag palette (`src/tui/component/shared/colored_tags.rs`), shared here
// so CLI tags render identically.
const TAG_HASH_SEED: u32 = 1870435234;
const TAG_PALETTE: [(u8, u8, u8); 14] = [
    (0xFF, 0xD6, 0xE0),
    (0xFF, 0xC9, 0xB9),
    (0xFF, 0xE0, 0xBD),
    (0xFB, 0xF0, 0xA8),
    (0xDC, 0xED, 0xC1),
    (0xB8, 0xE6, 0xD2),
    (0xAE, 0xE3, 0xE8),
    (0xBB, 0xDC, 0xF7),
    (0xC6, 0xCB, 0xF5),
    (0xD9, 0xC7, 0xF0),
    (0xEF, 0xC7, 0xE8),
    (0xFA, 0xF7, 0xF2),
    (0xD6, 0xD0, 0xC8),
    (0x5C, 0x54, 0x70),
];

/// The TUI's deterministic tag color.
pub fn tag_rgb(tag: &str) -> (u8, u8, u8) {
    let index = XxHash32::oneshot(TAG_HASH_SEED, tag.as_bytes()) as usize % TAG_PALETTE.len();
    TAG_PALETTE[index]
}
