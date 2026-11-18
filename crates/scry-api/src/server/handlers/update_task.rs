//! Handler for updating a task by handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use scry_core::ids::Handle;
use scry_core::models::{ExternalIdentity, ExternalMetadata, Priority, SectionId, Tags, TaskState};
use scry_core::temporal::When;
use serde::Deserialize;

use crate::error::AppError;
use crate::server::ScryState;
use crate::service::{SectionInput, TaskPatch};

/// Request body for updating a task; omitted fields are left unchanged.
#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub section_id: Option<SectionId>,
    pub new_section: Option<String>,
    #[serde(default)]
    pub clear_section: bool,
    pub state: Option<TaskState>,
    #[serde(default, with = "scry_core::sync::double_option")]
    pub due: Option<Option<When>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "scry_core::sync::double_option"
    )]
    pub external_identity: Option<Option<ExternalIdentity>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "scry_core::sync::double_option"
    )]
    pub external_metadata: Option<Option<ExternalMetadata>>,
}

/// Update a task from the request body.
pub async fn update_task(
    State(state): State<ScryState>,
    Path(handle): Path<Handle>,
    Json(body): Json<UpdateTaskRequest>,
) -> Response {
    let task = match state.service.get_task_by_handle(&handle).await {
        Ok(task) => task,
        Err(error) => return AppError::from(error).into(),
    };

    let section_choices = [
        body.section_id.is_some(),
        body.new_section.is_some(),
        body.clear_section,
    ];
    if section_choices.into_iter().filter(|chosen| *chosen).count() > 1 {
        return AppError::Usage(
            "specify at most one of section_id, new_section, or clear_section".to_string(),
        )
        .into();
    }

    let section = if let Some(section_id) = body.section_id {
        SectionInput::Existing(section_id)
    } else if let Some(name) = body.new_section {
        SectionInput::New(name)
    } else if body.clear_section {
        SectionInput::NoSection
    } else {
        SectionInput::Unchanged
    };

    let input = TaskPatch {
        title: body.title,
        description: body.description,
        priority: body.priority,
        tags: body.tags.map(Tags::from),
        section,
        state: body.state,
        due: body.due,
        external_identity: body.external_identity,
        external_metadata: body.external_metadata,
    };

    match state.service.update_task_by_id(task.id, input).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => AppError::from(error).into(),
    }
}

#[cfg(test)]
mod tests {
    // These tests guard the three-state contract of `due`, `external_identity`,
    // and `external_metadata`. They rely on `double_option` and `#[serde(default)]`
    // together: without `default`, serde treats a field with a custom
    // deserializer as required, so an omitted field fails to parse and axum
    // turns that into a 422 at the REST boundary. `scry sync` sends only
    // `external_metadata` and omits the other two, so dropping `default` breaks
    // every sync update rather than just an edge case.

    use super::*;
    use serde_json::json;

    #[test]
    fn omitted_tri_state_fields_deserialize_as_unchanged() {
        let request: UpdateTaskRequest = serde_json::from_value(json!({}))
            .expect("omitted tri-state fields should mean unchanged");

        assert!(request.due.is_none());
        assert!(request.external_identity.is_none());
        assert!(request.external_metadata.is_none());
    }

    #[test]
    fn tri_state_fields_distinguish_absent_and_null() {
        // An explicit null is a request to clear, not the same as omitting the field.
        let request: UpdateTaskRequest =
            serde_json::from_value(json!({ "due": null, "external_identity": null }))
                .expect("explicit null should clear the fields");

        assert_eq!(request.due, Some(None));
        assert_eq!(request.external_identity, Some(None));

        // The shape `scry sync` sends: metadata present, due and identity absent.
        let request: UpdateTaskRequest = serde_json::from_value(json!({
            "external_metadata": { "provider": "github", "state": "open" }
        }))
        .expect("metadata without due or identity should still parse");

        assert!(matches!(
            request.external_metadata,
            Some(Some(ExternalMetadata::Github { .. }))
        ));
    }
}
