use anstyle::{AnsiColor, Style};
use clap::ValueEnum;
use scry_core::models::{Color, Priority};

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
        Some(color) => paint(
            Style::new().fg_color(Some(AnsiColor::from(color).into())),
            text,
        ),
        None => text.to_string(),
    }
}

/// The short priority label (`pN`).
pub fn priority(priority: Priority) -> String {
    paint(
        Style::new().fg_color(Some(AnsiColor::from(priority.color()).into())),
        &format!("p{}", i64::from(priority)),
    )
}

/// The long priority label (`pN - Name`).
pub fn priority_long(priority: Priority) -> String {
    paint(
        Style::new().fg_color(Some(AnsiColor::from(priority.color()).into())),
        &format!("p{} - {}", i64::from(priority), priority),
    )
}

