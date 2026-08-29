use crate::models::TaskID;
use crate::tui::component::RenderContext;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, ToSpan};

pub struct Hints;

impl Hints {
    pub fn new() -> Self {
        Hints
    }

    pub fn render(&self, ctx: &mut RenderContext, selected_task_id: Option<TaskID>) {
        // each "hint" is a Text, since parts are styled differently
        let mut hints = vec![];

        // initialize hints with static, always-available actions
        hints.push(vec!["[/] ".to_span(), "command".dim()]);
        hints.push(vec!["[f]".to_span(), "ilter".dim()]);
        hints.push(vec!["[a]".to_span(), "dd".dim()]);

        // add select actions if any task is selected
        if selected_task_id.is_some() {
            hints.push(vec!["[e]".to_span(), "dit".dim()]);
            hints.push(vec!["[d]".to_span(), "elete".dim()]);
            hints.push(vec!["[n]".to_span(), "ote".dim()]);
        }

        // try to find the status of the selected task, if one is selected
        let selected_status_id = selected_task_id
            .and_then(|id| ctx.state.get_task_by_id(id))
            .map(|task| task.status_id);

        // if there's a status before this one, we will suggest an action for moving it
        if let Some(previous_status_name) = selected_status_id
            .and_then(|id| ctx.state.previous_status(id))
            .map(|s| &s.name)
        {
            hints.push(vec![
                "[<]".to_span(),
                format!(" move to {previous_status_name}").dim(),
            ]);
        }
        // if there's a status after this one, we will also suggest an action for moving it
        if let Some(next_status_name) = selected_status_id
            .and_then(|id| ctx.state.next_status(id))
            .map(|s| &s.name)
        {
            hints.push(vec![
                "[>]".to_span(),
                format!(" move to {next_status_name}").dim(),
            ]);
        }

        let spans: Vec<Span<'_>> =
            itertools::Itertools::intersperse(hints.into_iter(), vec![" • ".dim()])
                .flatten()
                .collect();

        ctx.render(Line::from(spans).left_aligned());
    }
}
