//! Handler for moving a section to a new position.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::models::SectionId;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::SectionPlacement;

/// Placement in a section move request.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveSectionPlacement {
    Top,
    Bottom,
    Before,
    After,
}

/// Request body for moving a section.
#[derive(Deserialize)]
pub struct MoveSectionRequest {
    pub placement: MoveSectionPlacement,
    #[serde(default)]
    pub anchor_id: Option<SectionId>,
}

fn resolve_move_request(body: MoveSectionRequest) -> Result<SectionPlacement, AppError> {
    match (body.placement, body.anchor_id) {
        (MoveSectionPlacement::Top, None) => Ok(SectionPlacement::Top),
        (MoveSectionPlacement::Bottom, None) => Ok(SectionPlacement::Bottom),
        (MoveSectionPlacement::Before, Some(anchor_id)) => Ok(SectionPlacement::Before(anchor_id)),
        (MoveSectionPlacement::After, Some(anchor_id)) => Ok(SectionPlacement::After(anchor_id)),
        (MoveSectionPlacement::Top | MoveSectionPlacement::Bottom, Some(_)) => Err(
            AppError::Usage("top and bottom placements must not include anchor_id".to_string()),
        ),
        (MoveSectionPlacement::Before | MoveSectionPlacement::After, None) => Err(AppError::Usage(
            "before and after placements require anchor_id".to_string(),
        )),
    }
}

/// Move a section and return the updated section.
pub async fn move_section(
    State(state): State<ScryState>,
    Path(id): Path<SectionId>,
    Json(body): Json<MoveSectionRequest>,
) -> Response {
    let placement = match resolve_move_request(body) {
        Ok(placement) => placement,
        Err(error) => return error.into(),
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
        assert!(matches!(
            resolve_move_request(top),
            Ok(SectionPlacement::Top)
        ));

        let anchor_id = Uuid::new_v4();
        let before: MoveSectionRequest =
            serde_json::from_value(json!({ "placement": "before", "anchor_id": anchor_id }))
                .expect("before with an anchor is valid");
        assert!(matches!(
            resolve_move_request(before),
            Ok(SectionPlacement::Before(parsed)) if parsed == anchor_id
        ));
    }

    #[test]
    fn rejects_the_legacy_op_tag() {
        assert!(
            serde_json::from_value::<MoveSectionRequest>(json!({ "op": "top" })).is_err(),
            "the request body must use placement"
        );
    }

    #[test]
    fn rejects_an_anchor_on_top_or_bottom() {
        let anchor_id = Uuid::new_v4();

        let top: MoveSectionRequest =
            serde_json::from_value(json!({ "placement": "top", "anchor_id": anchor_id }))
                .expect("top with an anchor still parses");
        assert!(matches!(resolve_move_request(top), Err(AppError::Usage(_))));

        let bottom: MoveSectionRequest =
            serde_json::from_value(json!({ "placement": "bottom", "anchor_id": anchor_id }))
                .expect("bottom with an anchor still parses");
        assert!(matches!(
            resolve_move_request(bottom),
            Err(AppError::Usage(_))
        ));
    }

    #[test]
    fn before_and_after_require_an_anchor() {
        let before: MoveSectionRequest = serde_json::from_value(json!({ "placement": "before" }))
            .expect("before without an anchor still parses");
        assert!(matches!(
            resolve_move_request(before),
            Err(AppError::Usage(_))
        ));

        let after: MoveSectionRequest = serde_json::from_value(json!({ "placement": "after" }))
            .expect("after without an anchor still parses");
        assert!(matches!(
            resolve_move_request(after),
            Err(AppError::Usage(_))
        ));
    }
}
