mod new_task_input;
mod popup;
mod status_bar;
mod task_list;
mod title_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // task list
            Constraint::Length(3), // new task input
            Constraint::Length(1), // status bar
        ])
        .split(area);

    title_bar::render(frame, &app.project.name, layout[0]);
    task_list::render(frame, app, layout[1]);
    new_task_input::render(frame, &app.input, layout[2], app.is_input_selected());
    status_bar::render(frame, app, layout[3]);

    popup::render(frame, app);
}
