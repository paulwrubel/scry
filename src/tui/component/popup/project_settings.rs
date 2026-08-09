use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::models::STATE_COLORS;
use crate::tui::action::Action;
use crate::tui::component::{AppContext, Component};

struct EditField {
    buffer: String,
    cursor_position: usize,
}

impl EditField {
    fn new(text: String) -> Self {
        let len = text.len();
        Self {
            buffer: text,
            cursor_position: len,
        }
    }
    fn empty() -> Self {
        Self {
            buffer: String::new(),
            cursor_position: 0,
        }
    }
}

enum SettingsMode {
    Browsing,
    EditingName {
        field: EditField,
    },
    AddingState {
        field: EditField,
    },
    PickingColor {
        state_id: i64,
        selected_color_index: usize,
    },
}

pub struct ProjectSettings {
    // ── internal ──
    selected_row: usize,
    mode: SettingsMode,
    delete_confirm: bool,
    state_count: usize,
}

impl ProjectSettings {
    pub fn new(state_count: usize) -> Self {
        Self {
            selected_row: 0,
            mode: SettingsMode::Browsing,
            delete_confirm: false,
            state_count,
        }
    }

    pub fn sync(&mut self, ctx: &AppContext) {
        self.state_count = ctx.states.len();
        if self.selected_row > self.state_count {
            self.selected_row = self.state_count;
        }
    }
}

impl Component for ProjectSettings {
    fn handle_event(&mut self, ctx: &AppContext, key: KeyEvent) -> Option<Action> {
        // take the mode out of self so each arm can reassign it while dispatching
        let mode = std::mem::replace(&mut self.mode, SettingsMode::Browsing);
        match mode {
            SettingsMode::Browsing => match key.code {
                KeyCode::Esc => Some(Action::DismissPopup),
                KeyCode::Up => {
                    self.selected_row = self.selected_row.saturating_sub(1);
                    self.delete_confirm = false;
                    None
                }
                KeyCode::Down => {
                    self.selected_row = self.selected_row.saturating_add(1).min(self.state_count);
                    self.delete_confirm = false;
                    None
                }
                KeyCode::Char('r') => {
                    let name = if self.selected_row == 0 {
                        ctx.project.name.clone()
                    } else {
                        ctx.states
                            .get(self.selected_row - 1)
                            .map(|s| s.name.clone())
                            .unwrap_or_default()
                    };
                    self.mode = SettingsMode::EditingName {
                        field: EditField::new(name),
                    };
                    None
                }
                KeyCode::Char('a') => {
                    self.mode = SettingsMode::AddingState {
                        field: EditField::empty(),
                    };
                    None
                }
                KeyCode::Char('d') => {
                    if self.selected_row == 0 {
                        return None;
                    }
                    if !self.delete_confirm {
                        self.delete_confirm = true;
                        return None;
                    }
                    ctx.states
                        .get(self.selected_row - 1)
                        .map(|s| Action::DeleteState(s.name.clone()))
                }
                KeyCode::Char('c') => {
                    if self.selected_row > 0 {
                        if let Some(state) = ctx.states.get(self.selected_row - 1) {
                            let selected_color_index = state
                                .color
                                .as_ref()
                                .map(|c| c.0.as_str())
                                .and_then(|c| STATE_COLORS.iter().position(|(name, _)| *name == c))
                                .map(|pos| pos + 1)
                                .unwrap_or(0);
                            self.mode = SettingsMode::PickingColor {
                                state_id: state.id,
                                selected_color_index,
                            };
                        }
                    }
                    None
                }
                KeyCode::Char('k') => {
                    if self.selected_row > 0 {
                        if let Some(state) = ctx.states.get(self.selected_row - 1) {
                            if state.position > 0 {
                                return Some(Action::ReorderState {
                                    state_name: state.name.clone(),
                                    new_position: state.position - 1,
                                });
                            }
                        }
                    }
                    None
                }
                KeyCode::Char('j') => {
                    if self.selected_row > 0 {
                        if let Some(state) = ctx.states.get(self.selected_row - 1) {
                            if state.position < self.state_count as i32 - 1 {
                                return Some(Action::ReorderState {
                                    state_name: state.name.clone(),
                                    new_position: state.position + 1,
                                });
                            }
                        }
                    }
                    None
                }
                _ => None,
            },
            SettingsMode::EditingName { mut field } => match key.code {
                KeyCode::Esc => {
                    self.mode = SettingsMode::Browsing;
                    None
                }
                KeyCode::Enter => {
                    let trimmed = field.buffer.trim().to_string();
                    if trimmed.is_empty() {
                        self.mode = SettingsMode::Browsing;
                        return None;
                    }
                    if self.selected_row == 0 {
                        if trimmed != ctx.project.name {
                            self.mode = SettingsMode::Browsing;
                            return Some(Action::RenameProject(trimmed));
                        }
                    } else if let Some(old) = ctx
                        .states
                        .get(self.selected_row - 1)
                        .map(|s| s.name.clone())
                    {
                        if trimmed != old {
                            self.mode = SettingsMode::Browsing;
                            return Some(Action::RenameState {
                                old_name: old,
                                new_name: trimmed,
                            });
                        }
                    }
                    self.mode = SettingsMode::Browsing;
                    None
                }
                KeyCode::Char(c) => {
                    field.buffer.insert(field.cursor_position, c);
                    field.cursor_position += 1;
                    self.mode = SettingsMode::EditingName { field };
                    None
                }
                KeyCode::Backspace => {
                    if field.cursor_position > 0 {
                        field.cursor_position -= 1;
                        field.buffer.remove(field.cursor_position);
                    }
                    self.mode = SettingsMode::EditingName { field };
                    None
                }
                KeyCode::Left => {
                    if field.cursor_position > 0 {
                        field.cursor_position -= 1;
                    }
                    self.mode = SettingsMode::EditingName { field };
                    None
                }
                KeyCode::Right => {
                    if field.cursor_position < field.buffer.len() {
                        field.cursor_position += 1;
                    }
                    self.mode = SettingsMode::EditingName { field };
                    None
                }
                _ => {
                    self.mode = SettingsMode::EditingName { field };
                    None
                }
            },
            SettingsMode::AddingState { mut field } => match key.code {
                KeyCode::Esc => {
                    self.mode = SettingsMode::Browsing;
                    None
                }
                KeyCode::Enter => {
                    let trimmed = field.buffer.trim().to_string();
                    self.mode = SettingsMode::Browsing;
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(Action::AddState(trimmed))
                    }
                }
                KeyCode::Char(c) => {
                    field.buffer.insert(field.cursor_position, c);
                    field.cursor_position += 1;
                    self.mode = SettingsMode::AddingState { field };
                    None
                }
                KeyCode::Backspace => {
                    if field.cursor_position > 0 {
                        field.cursor_position -= 1;
                        field.buffer.remove(field.cursor_position);
                    }
                    self.mode = SettingsMode::AddingState { field };
                    None
                }
                KeyCode::Left => {
                    if field.cursor_position > 0 {
                        field.cursor_position -= 1;
                    }
                    self.mode = SettingsMode::AddingState { field };
                    None
                }
                KeyCode::Right => {
                    if field.cursor_position < field.buffer.len() {
                        field.cursor_position += 1;
                    }
                    self.mode = SettingsMode::AddingState { field };
                    None
                }
                _ => {
                    self.mode = SettingsMode::AddingState { field };
                    None
                }
            },
            SettingsMode::PickingColor {
                state_id,
                selected_color_index,
            } => match key.code {
                KeyCode::Esc => {
                    self.mode = SettingsMode::Browsing;
                    None
                }
                KeyCode::Up => {
                    let new_index = selected_color_index.saturating_sub(1);
                    self.mode = SettingsMode::PickingColor {
                        state_id,
                        selected_color_index: new_index,
                    };
                    None
                }
                KeyCode::Down => {
                    let new_index = selected_color_index
                        .saturating_add(1)
                        .min(STATE_COLORS.len());
                    self.mode = SettingsMode::PickingColor {
                        state_id,
                        selected_color_index: new_index,
                    };
                    None
                }
                KeyCode::Enter => {
                    let color_name = if selected_color_index == 0 {
                        None
                    } else {
                        STATE_COLORS
                            .get(selected_color_index - 1)
                            .map(|(name, _)| name.to_string())
                    };
                    self.mode = SettingsMode::Browsing;
                    Some(Action::SetStateColor {
                        state_id,
                        color: color_name,
                    })
                }
                _ => {
                    self.mode = SettingsMode::PickingColor {
                        state_id,
                        selected_color_index,
                    };
                    None
                }
            },
        }
    }

    fn render(&self, ctx: &AppContext, frame: &mut Frame, area: Rect) {
        // PickingColor mode renders side by side so the color options stay visible
        if let SettingsMode::PickingColor {
            selected_color_index,
            ..
        } = &self.mode
        {
            let color_idx = *selected_color_index;

            // ── left panel: compact settings view ──
            let mut left: Vec<Line> = Vec::new();
            left.push(Line::from(Span::styled(
                "  Project",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            left.push(Line::from(Span::styled(
                format!("    {}", ctx.project.name),
                Style::default(),
            )));
            left.push(Line::from(""));
            left.push(Line::from(Span::styled(
                "  States",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for (i, state) in ctx.states.iter().enumerate() {
                let row_idx = i + 1;
                let is_selected = row_idx == self.selected_row;
                let marker = if is_selected { ">" } else { " " };
                let color_name = state.color.as_ref().map(|c| c.0.as_str()).unwrap_or("none");
                left.push(Line::from(format!(
                    "  {} {}. {} [{}]",
                    marker, row_idx, state.name, color_name
                )));
            }
            if ctx.states.is_empty() {
                left.push(Line::from("    (no states)"));
            }

            // ── right panel: color picker ──
            let mut right: Vec<Line> = Vec::new();
            right.push(Line::from(Span::styled(
                "  Color",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            right.push(Line::from(""));

            let is_none_selected = color_idx == 0;
            let marker = if is_none_selected { ">" } else { " " };
            let style = if is_none_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            right.push(Line::from(Span::styled(
                format!("  {} None (default)", marker),
                style,
            )));

            for (i, (name, color)) in STATE_COLORS.iter().enumerate() {
                let cidx = i + 1;
                let is_selected = cidx == color_idx;
                let marker = if is_selected { ">" } else { " " };
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(*color)
                };
                right.push(Line::from(Span::styled(
                    format!("  {} {}", marker, name),
                    style,
                )));
            }

            // ── merge side by side ──
            let left_width = 34usize;
            let max_rows = left.len().max(right.len());
            let mut lines: Vec<Line> = Vec::new();

            for row in 0..max_rows {
                let left_text = if row < left.len() {
                    let line_str = left[row]
                        .spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<Vec<&str>>()
                        .join("");
                    format!("{:<width$}", line_str, width = left_width)
                } else {
                    " ".repeat(left_width)
                };

                let right_spans = if row < right.len() {
                    right[row].spans.clone()
                } else {
                    vec![Span::raw("")]
                };

                let mut all_spans = vec![Span::raw(left_text), Span::raw(" ")];
                all_spans.extend(right_spans);
                lines.push(Line::from(all_spans));
            }

            let height = (lines.len() + 2) as u16;
            let width = 68u16;
            let title = format!(" Project Settings: {}", ctx.project.name);
            render_centered_popup(frame, lines, height.min(area.height), width, &title);
            return;
        }

        // single-column layout for browsing and editing modes
        let mut lines: Vec<Line> = Vec::new();

        // Project section
        lines.push(Line::from(Span::styled(
            "  Project",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        let row0_selected = self.selected_row == 0;

        match &self.mode {
            SettingsMode::EditingName { field } if self.selected_row == 0 => {
                let text = format!("    Name:  {}", field.buffer);
                let style = if row0_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
                lines.push(Line::from(Span::styled(
                    "           Enter confirm  Esc cancel",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
            _ => {
                let text = format!("    Name:  {}", ctx.project.name);
                let style = if row0_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
                lines.push(Line::from(Span::styled(
                    "           (r) rename",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }

        lines.push(Line::from(""));

        // States section
        lines.push(Line::from(Span::styled(
            "  States",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        if let SettingsMode::AddingState { field } = &self.mode {
            let text = format!("    New:  {}", field.buffer);
            let style = if self.selected_row == 1 {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
            lines.push(Line::from(Span::styled(
                "          Enter confirm  Esc cancel",
                Style::default().add_modifier(Modifier::DIM),
            )));
        } else {
            for (i, state) in ctx.states.iter().enumerate() {
                let row_idx = i + 1;
                let is_selected = row_idx == self.selected_row;
                let is_editing =
                    matches!(&self.mode, SettingsMode::EditingName { .. }) && is_selected;

                if is_editing {
                    if let SettingsMode::EditingName { field } = &self.mode {
                        let text = format!("    {}. {}", i + 1, field.buffer);
                        let style = Style::default().add_modifier(Modifier::REVERSED);
                        lines.push(Line::from(Span::styled(text, style)));
                        lines.push(Line::from(Span::styled(
                            "        Enter confirm  Esc cancel",
                            Style::default().add_modifier(Modifier::DIM),
                        )));
                    }
                } else {
                    let marker = if is_selected { ">" } else { " " };
                    let state_color_name = state.color.as_ref().map(|c| c.0.as_str());

                    let color_text = match state_color_name {
                        Some(name) => format!("[{}]", name),
                        None => "[none]".to_string(),
                    };

                    let rat_color = state.color.clone().map(ratatui::style::Color::from);

                    let color_style = if let Some(c) = rat_color {
                        Style::default().fg(c)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    };

                    let name_style = if is_selected {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };

                    let hints = if is_selected { "kj r c d" } else { "" };

                    let mut spans: Vec<Span> = Vec::new();
                    spans.push(Span::styled(
                        format!("  {} {}. ", marker, i + 1),
                        name_style,
                    ));
                    spans.push(Span::styled(format!("{:<20} ", state.name), name_style));
                    spans.push(Span::styled(
                        format!("{:<10}", color_text),
                        if is_selected { name_style } else { color_style },
                    ));
                    if is_selected {
                        spans.push(Span::styled(
                            format!(" {}", hints),
                            Style::default().add_modifier(Modifier::DIM),
                        ));
                    }

                    lines.push(Line::from(spans));
                }
            }
        }

        if ctx.states.is_empty() && !matches!(&self.mode, SettingsMode::AddingState { .. }) {
            lines.push(Line::from(Span::styled(
                "    (no states — press a to add)",
                Style::default().add_modifier(Modifier::DIM),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            Style::default().add_modifier(Modifier::DIM),
        )));
        lines.push(Line::from(Span::styled(
            "  Esc close  k/j reorder  r rename  c color  d delete",
            Style::default().add_modifier(Modifier::DIM),
        )));
        lines.push(Line::from(Span::styled(
            "  a add state",
            Style::default().add_modifier(Modifier::DIM),
        )));

        let height = (lines.len() + 2) as u16;
        let width = 62u16;
        let title = format!(" Project Settings: {}", ctx.project.name);

        // record the cursor position for the input field so the coordinator can show it
        if let SettingsMode::EditingName { field } | SettingsMode::AddingState { field } =
            &self.mode
        {
            let popup_area = centered_rect(width, height.min(area.height), area);

            let (input_line_idx, text_offset): (usize, u16) =
                if matches!(&self.mode, SettingsMode::AddingState { .. }) {
                    (5, 10) // "    New:  "
                } else if self.selected_row == 0 {
                    (1, 11) // "    Name:  "
                } else {
                    // editing field replaces the state row, so its line is 4 + selected_row
                    (
                        4 + self.selected_row,
                        6 + (self.selected_row.to_string().len() as u16), // "    {N}. "
                    )
                };

            let col = popup_area.x + 1 + text_offset + field.cursor_position as u16;
            let row = popup_area.y + 1 + input_line_idx as u16;
            frame.set_cursor_position((col, row));
        }

        let _ = render_centered_popup(frame, lines, height.min(area.height), width, &title);
    }
}

fn render_centered_popup(
    frame: &mut Frame,
    lines: Vec<Line>,
    height: u16,
    width: u16,
    title: &str,
) -> Rect {
    let area = frame.area();
    let popup_area = centered_rect(width, height, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(title);

    // clear the area behind the popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(popup_area)[1];

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner)[1];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    popup_area
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
            Constraint::Length(percent_y),
            Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
            Constraint::Length(percent_x),
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
        ])
        .split(popup_layout[1])[1]
}
