use crate::models::TaskID;
use crate::tui::component::{ProjectStatusTasks, RenderContext};
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

    pub fn render(&self, ctx: &mut RenderContext, selected_task_id: Option<TaskID>) {
        // initize hints with status, always-available actions
        let mut actions: Vec<[String; 2]> = vec![
            ["[/]", " commands"].map(|s| s.to_string()),
            ["[a]", "dd task"].map(|s| s.to_string()),
        ];

        // add delete actions if any task is selected
        if selected_task_id.is_some() {
            actions.push(["[d]", "elete"].map(|s| s.to_string()));
        }

        // try to find the status of the selected task, if one is selected
        let project_status_tasks = ProjectStatusTasks::from(ctx.state);
        let selected_status_id = selected_task_id
            .and_then(|id| project_status_tasks.get_task_by_id(id))
            .map(|t| t.status_id);

        // if there's a status before this one, we will suggest an action for moving it
        if let Some(previous_status_name) = selected_status_id
            .and_then(|id| project_status_tasks.previous_status(id))
            .map(|s| &s.name)
        {
            actions.push([
                "[<]".to_string(),
                format!(" move to {previous_status_name}"),
            ]);
        }
        // if there's a status after this one, we will also suggest an action for moving it
        if let Some(next_status_name) = selected_status_id
            .and_then(|id| project_status_tasks.next_status(id))
            .map(|s| &s.name)
        {
            actions.push(["[>]".to_string(), format!(" move to {next_status_name}")]);
        }

        let spans: Vec<Span<'_>> = if self.message.is_empty() {
            actions
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

        ctx.render(Line::from(spans).left_aligned());
    }
}
