//! Handler for moving a section to a new position.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::SectionPlacement;

/// Request body for moving a section; the `placement` tag selects the placement.
#[derive(Deserialize)]
#[serde(tag = "placement", rename_all = "snake_case")]
pub enum MoveSectionRequest {
    Top,
    Bottom,
    Before { anchor_id: SectionId },
    After { anchor_id: SectionId },
}

/// Move a section and return the updated section.
pub async fn move_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<MoveSectionRequest>,
) -> Response {
    let placement = match body {
        MoveSectionRequest::Top => SectionPlacement::Top,
        MoveSectionRequest::Bottom => SectionPlacement::Bottom,
        MoveSectionRequest::Before { anchor_id } => SectionPlacement::Before(anchor_id),
        MoveSectionRequest::After { anchor_id } => SectionPlacement::After(anchor_id),
    };

    match state.service.move_section(id, placement).await {
        Ok(section) => Json(section).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn parses_the_placement_tag() {
        let top: MoveSectionRequest =
            serde_json::from_value(json!({ "placement": "top" })).expect("top is valid");
        assert!(matches!(top, MoveSectionRequest::Top));

        let anchor_id = Uuid::new_v4();
        let before: MoveSectionRequest =
            serde_json::from_value(json!({ "placement": "before", "anchor_id": anchor_id }))
                .expect("before with an anchor is valid");
        match before {
            MoveSectionRequest::Before { anchor_id: parsed } => assert_eq!(parsed, anchor_id),
            _ => panic!("expected a before placement"),
        }
    }

    #[test]
    fn rejects_the_legacy_op_tag() {
        assert!(
            serde_json::from_value::<MoveSectionRequest>(json!({ "op": "top" })).is_err(),
            "the request body must use placement"
        );
    }
}
