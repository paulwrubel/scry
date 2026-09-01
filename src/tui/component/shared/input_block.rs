use crate::tui::component::{ProjectState, RenderContext};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::{CursorMove, TextArea, WrapMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    SingleLine,
    Viewing,
    Editing,
}

#[derive(Debug, Clone)]
pub struct InputBlock {
    pub is_focused: bool,
    title: Option<String>,
    mode: InputMode,

    textarea: TextArea<'static>,
}

impl InputBlock {
    pub fn new(is_focused: bool, is_multiline: bool) -> Self {
        Self {
            is_focused,
            title: None,
            mode: if is_multiline {
                InputMode::Viewing
            } else {
                InputMode::SingleLine
            },

            textarea: TextArea::default(),
        }
    }

    pub fn with_title(self, title: String) -> Self {
        Self {
            title: Some(title),
            ..self
        }
    }

    pub fn with_placeholder_text(mut self, placeholder_text: String) -> Self {
        self.textarea
            .set_styled_placeholder(Text::from(placeholder_text).dim().italic());
        self
    }

    pub fn with_text(self, text: String) -> Self {
        let mut textarea = self.textarea.clone();
        textarea.clear();
        textarea.insert_str(text);
        textarea.move_cursor(CursorMove::End);
        Self { textarea, ..self }
    }

    pub fn focus(&mut self) {
        self.textarea.move_cursor(CursorMove::End);
        self.mode = match &self.mode {
            InputMode::Viewing | InputMode::Editing => InputMode::Viewing,
            InputMode::SingleLine => InputMode::SingleLine,
        };
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.mode = match &self.mode {
            InputMode::Viewing | InputMode::Editing => InputMode::Viewing,
            InputMode::SingleLine => InputMode::SingleLine,
        };
        self.is_focused = false;
    }

    pub fn is_editing(&self) -> bool {
        self.mode == InputMode::Editing
    }

    pub fn buffer_text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn handle_event(&mut self, _state: &ProjectState, key: KeyEvent) {
        if !self.is_focused {
            return;
        }

        match (key.modifiers, key.code, &self.mode) {
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Enter, InputMode::Viewing) => {
                self.mode = InputMode::Editing;
            }
            (_, KeyCode::Esc, InputMode::Editing) => {
                self.mode = InputMode::Viewing;
            }
            // single-line inputs never receive newline keys
            (_, KeyCode::Enter, InputMode::SingleLine) => {}
            // in normal mode a multiline input only responds to Enter (above);
            // everything else bubbles so the parent can change focus
            (_, _, InputMode::Viewing) => {}
            // anything else, we send to the textarea
            (KeyModifiers::NONE | KeyModifiers::SHIFT, _, _) => {
                self.textarea.input(key);
            }
            _ => {}
        }
    }

    pub fn render(&self, ctx: &mut RenderContext) {
        let (border_style, text_style) = match (self.is_focused, self.mode) {
            (true, InputMode::Viewing) => (Style::default(), Style::default()),
            (true, _) => (Style::default(), Style::default()),
            (false, _) => (Style::default().dim(), Style::default().dim()),
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);
        if let Some(title) = &self.title {
            block = block.title(title.as_str());
        }
        if self.mode == InputMode::Editing {
            block = block.title_bottom(Line::from(" [Esc] to stop editing ").right_aligned())
        } else if self.is_focused && self.mode == InputMode::Viewing {
            block = block.title_bottom(Line::from(" [Enter] to edit ").right_aligned())
        }

        let mut textarea = self.textarea.clone();
        textarea.set_block(block);
        textarea.set_style(text_style);
        if !self.is_focused || self.mode == InputMode::Viewing {
            // no cursor
            textarea.set_cursor_line_style(Style::default());
            textarea.set_cursor_style(Style::default());
        }
        if matches!(self.mode, InputMode::Viewing | InputMode::Editing) {
            textarea.set_wrap_mode(WrapMode::Glyph);
        }

        ctx.render(&textarea);
    }
}
