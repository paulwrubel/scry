//! `show`.

use crate::client::{Client, TaskDetails};
use crate::color;
use crate::error::AppError;
use crate::output::{print_json, section_label};
use crate::state::format_local;
use scry_core::models::{Section, TaskState};

/// Show one task with its comments.
pub async fn show(client: &Client, handle: &str, output_json: bool) -> Result<(), AppError> {
    let detail: TaskDetails = client
        .get_json(
            &["tasks", handle],
            &[("detailed", "true"), ("include_handle", "true")],
        )
        .await?;

    if output_json {
        print_json(&detail)?;
        return Ok(());
    }

    let section: Option<Section> = match detail.task.section_id {
        Some(section_id) => {
            let section_id = section_id.to_string();
            client
                .get_json(&["sections", section_id.as_str()], &[])
                .await?
        }
        None => None,
    };

    let task = &detail.task;
    let display_handle = detail.handle.as_deref().unwrap_or(handle);

    let state_label = task.state.as_str().to_string();

    println!("{} #{}", task.title, display_handle);
    println!();

    if let Some(description) = &task.description {
        for line in description.lines() {
            println!("    {}", line);
        }
    }
    println!();

    anstream::println!("Priority:    {}", task.priority.as_str());
    anstream::println!(
        "Section:     {}",
        color::status(
            section.as_ref().and_then(|section| section.color),
            section_label(&section)
        )
    );
    anstream::println!("State:       {}", state_label);
    if let (TaskState::Closed, Some(reason)) = (task.state, task.closed_reason) {
        let closed = match task.closed_at {
            Some(closed_at) => format!("{} at {}", reason.as_str(), format_local(closed_at)),
            None => reason.as_str().to_string(),
        };
        anstream::println!("Closed:      {}", closed);
    }
    let tags = task
        .tags
        .iter()
        .map(|tag| tag.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    anstream::println!("Tags:        {}", tags);
    println!("Created at:  {}", format_local(task.created_at));
    println!("Updated at:  {}", format_local(task.updated_at));
    println!();

    for comment in detail.comments.unwrap_or_default() {
        let comment_created_at = format_local(comment.created_at);
        let comment_updated_at = format_local(comment.updated_at);
        println!("Created at:  {}", comment_created_at);
        println!("Updated at:  {}", comment_updated_at);
        println!("{}", comment.contents);
        println!();
    }

    Ok(())
}
