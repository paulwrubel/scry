mod new_task_input;
mod popup;
mod status_bar;
mod task_list;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let project_name = app.project.name.clone();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" scry ")
        .title(format!(" {} ", project_name));
    let inner_area = block.inner(area);

    let h_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1), // left padding
            Constraint::Min(0),    // content
            Constraint::Length(1), // right padding
        ])
        .split(inner_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing
            Constraint::Min(0),    // task list
            Constraint::Length(3), // new task input
            Constraint::Length(1), // status bar
        ])
        .split(h_layout[1]);

    task_list::render(frame, app, layout[1]);
    new_task_input::render(frame, &app.input, layout[2], app.is_input_selected());
    status_bar::render(frame, app, layout[3]);

    frame.render_widget(block, area);

    popup::render(frame, app);
}
