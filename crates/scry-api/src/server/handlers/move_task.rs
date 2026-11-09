//! Handler for moving a task to a section.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use scry_core::models::{SectionId, TaskId};
use serde::{Deserialize, Deserializer};

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::{SectionInput, TaskPlacement};

/// Placement in a task move request.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveTaskPlacement {
    Top,
    Bottom,
    Before,
    After,
}

/// Request body for moving a task.
///
/// `section_id` has three states: omitted keeps the current section, `null`
/// moves to No section, and an id moves to that section.
#[derive(Deserialize)]
pub struct MoveTaskRequest {
    placement: MoveTaskPlacement,
    #[serde(default, deserialize_with = "deserialize_section_id")]
    section_id: Option<Option<SectionId>>,
    #[serde(default)]
    anchor_id: Option<TaskId>,
}

fn deserialize_section_id<'de, D>(deserializer: D) -> Result<Option<Option<SectionId>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<SectionId>::deserialize(deserializer).map(Some)
}

fn resolve_move_request(body: MoveTaskRequest) -> Result<(SectionInput, TaskPlacement), AppError> {
    let section = match body.section_id {
        Some(Some(id)) => SectionInput::Existing(id),
        Some(None) => SectionInput::NoSection,
        None => SectionInput::Unchanged,
    };

    let placement = match (body.placement, body.anchor_id) {
        (MoveTaskPlacement::Top, None) => TaskPlacement::Top,
        (MoveTaskPlacement::Bottom, None) => TaskPlacement::Bottom,
        (MoveTaskPlacement::Before, Some(anchor_id)) => TaskPlacement::Before(anchor_id),
        (MoveTaskPlacement::After, Some(anchor_id)) => TaskPlacement::After(anchor_id),
        (MoveTaskPlacement::Top | MoveTaskPlacement::Bottom, Some(_)) => {
            return Err(AppError::Usage(
                "top and bottom placements must not include anchor_id".to_string(),
            ));
        }
        (MoveTaskPlacement::Before | MoveTaskPlacement::After, None) => {
            return Err(AppError::Usage(
                "before and after placements require anchor_id".to_string(),
            ));
        }
    };

    Ok((section, placement))
}

/// Move a task to a section (or no section) and a placement.
pub async fn move_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Json(body): Json<MoveTaskRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let (section, placement) = match resolve_move_request(body) {
        Ok(parts) => parts,
        Err(error) => return error.into(),
    };

    match state
        .service
        .move_task_by_id(task.id, section, Some(placement))
        .await
    {
        Ok(change) => Json(change.task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn section_id_distinguishes_omitted_null_and_uuid() {
        let section_id = Uuid::new_v4();

        let omitted: MoveTaskRequest = serde_json::from_value(json!({ "placement": "top" }))
            .expect("omitted section_id is valid");
        assert!(omitted.section_id.is_none());

        let null: MoveTaskRequest =
            serde_json::from_value(json!({ "placement": "top", "section_id": null }))
                .expect("null section_id is valid");
        assert!(matches!(null.section_id, Some(None)));

        let id: MoveTaskRequest =
            serde_json::from_value(json!({ "placement": "top", "section_id": section_id }))
                .expect("a uuid section_id is valid");
        assert!(matches!(id.section_id, Some(Some(parsed)) if parsed == section_id));
    }

    #[test]
    fn placement_is_required() {
        assert!(
            serde_json::from_value::<MoveTaskRequest>(json!({})).is_err(),
            "placement must be present"
        );
        assert!(
            serde_json::from_value::<MoveTaskRequest>(json!({ "section_id": null })).is_err(),
            "placement must be present even with section_id"
        );
    }

    #[test]
    fn before_and_after_require_an_anchor() {
        let before: MoveTaskRequest = serde_json::from_value(json!({ "placement": "before" }))
            .expect("before without an anchor still parses");
        assert!(matches!(
            resolve_move_request(before),
            Err(AppError::Usage(_))
        ));

        let after: MoveTaskRequest = serde_json::from_value(json!({ "placement": "after" }))
            .expect("after without an anchor still parses");
        assert!(matches!(
            resolve_move_request(after),
            Err(AppError::Usage(_))
        ));
    }

    #[test]
    fn top_and_bottom_reject_an_anchor() {
        let anchor_id = Uuid::new_v4();

        let top: MoveTaskRequest =
            serde_json::from_value(json!({ "placement": "top", "anchor_id": anchor_id }))
                .expect("top with an anchor still parses");
        assert!(matches!(resolve_move_request(top), Err(AppError::Usage(_))));

        let bottom: MoveTaskRequest =
            serde_json::from_value(json!({ "placement": "bottom", "anchor_id": anchor_id }))
                .expect("bottom with an anchor still parses");
        assert!(matches!(
            resolve_move_request(bottom),
            Err(AppError::Usage(_))
        ));
    }

    #[test]
    fn maps_a_before_placement_with_no_section() {
        let anchor_id = Uuid::new_v4();
        let body: MoveTaskRequest = serde_json::from_value(json!({
            "placement": "before",
            "section_id": null,
            "anchor_id": anchor_id,
        }))
        .expect("before with an anchor and no section is valid");

        let (section, placement) = resolve_move_request(body).expect("parts are valid");
        assert!(matches!(section, SectionInput::NoSection));
        assert!(matches!(placement, TaskPlacement::Before(parsed) if parsed == anchor_id));
    }

    #[test]
    fn omitted_section_leaves_the_section_unchanged() {
        let body: MoveTaskRequest = serde_json::from_value(json!({ "placement": "top" }))
            .expect("top without a section is valid");

        let (section, placement) = resolve_move_request(body).expect("parts are valid");
        assert!(matches!(section, SectionInput::Unchanged));
        assert!(matches!(placement, TaskPlacement::Top));
    }
}
