//! `move`.

use scry_core::models::{Section, SectionId, Task, TaskId};
use serde::Serialize;

use crate::client::{Client, TaskDetails};
use crate::error::AppError;
use crate::output::print_json;

/// Where to place a task relative to its siblings.
pub enum TaskMovePlacement<'a> {
    Top,
    Bottom,
    Before(&'a str),
    After(&'a str),
}

/// Input for the `move` command.
pub struct MoveTask<'a> {
    pub handle: &'a str,
    pub section: Option<&'a str>,
    pub no_section: bool,
    pub placement: Option<TaskMovePlacement<'a>>,
}

#[derive(Serialize)]
struct MoveTaskRequest {
    placement: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_id: Option<Option<SectionId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    anchor_id: Option<TaskId>,
}

/// Move a task to a section, or place it within its current group.
pub async fn move_task(
    client: &Client,
    input: MoveTask<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    if input.section.is_none() && !input.no_section && input.placement.is_none() {
        return Err(AppError::Usage(
            "Specify a section, --no-section, or a placement flag.".to_string(),
        ));
    }

    let detail: TaskDetails = client
        .get_json(&["tasks", input.handle], &[("include_handle", "true")])
        .await?;

    let project_id = detail.task.project_id.to_string();

    let section_id = match (input.section, input.no_section) {
        (Some(name), _) => {
            let section: Section = client
                .get_json(
                    &["projects", project_id.as_str(), "sections", "by-name", name],
                    &[],
                )
                .await?;
            Some(Some(section.id))
        }
        (None, true) => Some(None),
        (None, false) => None,
    };

    let (placement, anchor_handle) = placement_wire(input.placement.as_ref());
    let anchor_id = match anchor_handle {
        Some(handle) => Some(task_id_by_handle(client, handle).await?),
        None => None,
    };

    let task: Task = client
        .post_json(
            &["tasks", input.handle, "move"],
            &[],
            &MoveTaskRequest {
                placement,
                section_id,
                anchor_id,
            },
        )
        .await?;

    if output_json {
        print_json(&task)?;
    } else {
        let handle = detail.handle.as_deref().unwrap_or(input.handle);
        let destination = input.section.unwrap_or(if input.no_section {
            "No section"
        } else {
            "current section"
        });
        match &input.placement {
            None => println!("Moved task {} --> \"{}\"", handle, destination),
            Some(TaskMovePlacement::Top) => {
                println!("Moved task {} to the top of \"{}\"", handle, destination)
            }
            Some(TaskMovePlacement::Bottom) => {
                println!("Moved task {} to the bottom of \"{}\"", handle, destination)
            }
            Some(TaskMovePlacement::Before(anchor)) => {
                println!(
                    "Moved task {} before {} in \"{}\"",
                    handle, anchor, destination
                )
            }
            Some(TaskMovePlacement::After(anchor)) => {
                println!(
                    "Moved task {} after {} in \"{}\"",
                    handle, anchor, destination
                )
            }
        }
    }

    Ok(())
}

/// Convert the CLI placement to its wire name and optional anchor handle.
fn placement_wire<'a>(
    placement: Option<&'a TaskMovePlacement<'a>>,
) -> (&'static str, Option<&'a str>) {
    match placement {
        None => ("bottom", None),
        Some(TaskMovePlacement::Top) => ("top", None),
        Some(TaskMovePlacement::Bottom) => ("bottom", None),
        Some(TaskMovePlacement::Before(anchor)) => ("before", Some(anchor)),
        Some(TaskMovePlacement::After(anchor)) => ("after", Some(anchor)),
    }
}

async fn task_id_by_handle(client: &Client, handle: &str) -> Result<TaskId, AppError> {
    let detail: TaskDetails = client
        .get_json(&["tasks", handle], &[("include_handle", "true")])
        .await?;
    Ok(detail.task.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn absent_placement_appends() {
        let (placement, anchor) = placement_wire(None);
        assert_eq!(placement, "bottom");
        assert!(anchor.is_none());
    }

    #[test]
    fn placement_names_map_to_wire_values() {
        assert_eq!(placement_wire(Some(&TaskMovePlacement::Top)), ("top", None));
        assert_eq!(
            placement_wire(Some(&TaskMovePlacement::Bottom)),
            ("bottom", None)
        );
        assert_eq!(
            placement_wire(Some(&TaskMovePlacement::Before("abc"))),
            ("before", Some("abc"))
        );
        assert_eq!(
            placement_wire(Some(&TaskMovePlacement::After("abc"))),
            ("after", Some("abc"))
        );
    }

    #[test]
    fn request_omits_an_unchanged_section() {
        let value = serde_json::to_value(MoveTaskRequest {
            placement: "top",
            section_id: None,
            anchor_id: None,
        })
        .expect("request serializes");

        assert_eq!(value, json!({ "placement": "top" }));
    }

    #[test]
    fn request_serializes_no_section_as_null() {
        let value = serde_json::to_value(MoveTaskRequest {
            placement: "bottom",
            section_id: Some(None),
            anchor_id: None,
        })
        .expect("request serializes");

        assert_eq!(value, json!({ "placement": "bottom", "section_id": null }));
    }

    #[test]
    fn request_serializes_a_section_and_anchor() {
        let section_id = Uuid::new_v4();
        let anchor_id = Uuid::new_v4();
        let value = serde_json::to_value(MoveTaskRequest {
            placement: "before",
            section_id: Some(Some(section_id)),
            anchor_id: Some(anchor_id),
        })
        .expect("request serializes");

        assert_eq!(
            value,
            json!({
                "placement": "before",
                "section_id": section_id,
                "anchor_id": anchor_id,
            })
        );
    }
}
