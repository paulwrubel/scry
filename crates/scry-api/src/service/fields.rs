use jiff::Timestamp;
use scry_core::models::{
    ClosedReason, Comment, Entity, Event, ExternalMetadata, FieldValues, Project, Section, Task,
    TaskState,
};
use scry_core::sync::operations::{ColorChange as OperationColorChange, OperationKind};
use serde_json::{Map, Value, json};

pub(super) fn subject(op: &OperationKind) -> Entity {
    match op {
        OperationKind::CreateProject { id, .. }
        | OperationKind::UpdateProject { id, .. }
        | OperationKind::DeleteProject { id } => Entity::Project(*id),
        OperationKind::CreateSection { id, .. }
        | OperationKind::UpdateSection { id, .. }
        | OperationKind::MoveSection { id, .. }
        | OperationKind::DeleteSection { id } => Entity::Section(*id),
        OperationKind::CreateTask { id, .. }
        | OperationKind::UpdateTask { id, .. }
        | OperationKind::MoveTask { id, .. }
        | OperationKind::DeleteTask { id } => Entity::Task(*id),
        OperationKind::CreateComment { id, .. }
        | OperationKind::UpdateComment { id, .. }
        | OperationKind::DeleteComment { id } => Entity::Comment(*id),
        OperationKind::CreateAutomation { id, .. }
        | OperationKind::UpdateAutomation { id, .. }
        | OperationKind::DeleteAutomation { id } => Entity::Automation(*id),
        OperationKind::CreateEvent { id, .. }
        | OperationKind::UpdateEvent { id, .. }
        | OperationKind::DeleteEvent { id } => Entity::Event(*id),
    }
}

pub(super) fn task_field_updates(
    task: &mut Task,
    op: &OperationKind,
    updated_at: Timestamp,
) -> FieldValues {
    let mut fields = Map::new();

    match op {
        OperationKind::UpdateTask {
            title,
            description,
            priority,
            tags,
            state,
            closed_reason,
            closed_at,
            due,
            external_identity,
            external_metadata,
            ..
        } => {
            if let Some(title) = title {
                task.title = title.clone();
                fields.insert("title".into(), json!(title));
            }
            if let Some(description) = description {
                task.description = description.clone();
                fields.insert("description".into(), json!(description));
            }
            if let Some(priority) = priority {
                task.priority = *priority;
                fields.insert("priority".into(), json!(priority));
            }
            if let Some(tags) = tags {
                task.tags = tags.clone();
                fields.insert("tags".into(), json!(tags));
            }
            let previous_state = task.state;
            let previous_reason = task.closed_reason;
            let previous_at = task.closed_at;
            if let Some(state) = state {
                task.state = *state;
                fields.insert("state".into(), json!(state));
            }
            if task.state == TaskState::Closed {
                if let Some(reason) = closed_reason {
                    task.closed_reason = *reason;
                }
                if let Some(at) = closed_at {
                    task.closed_at = *at;
                }
                if state.is_some_and(|value| value != previous_state) {
                    task.closed_reason =
                        Some(task.closed_reason.unwrap_or(ClosedReason::Completed));
                    task.closed_at = Some(task.closed_at.unwrap_or(updated_at));
                }
            } else {
                task.closed_reason = None;
                task.closed_at = None;
            }
            let reason_applied = task.state == TaskState::Closed && closed_reason.is_some();
            if reason_applied || task.closed_reason != previous_reason {
                fields.insert("closed_reason".into(), json!(task.closed_reason));
            }
            let at_applied = task.state == TaskState::Closed && closed_at.is_some();
            if at_applied || task.closed_at != previous_at {
                fields.insert("closed_at".into(), json!(task.closed_at));
            }
            if let Some(due) = due {
                task.due = due.clone();
                fields.insert("due".into(), json!(due));
            }
            if let Some(identity) = external_identity {
                task.external_identity = identity.clone();
                fields.insert("external_identity".into(), json!(identity));
            }
            if let Some(metadata) = external_metadata {
                task.external_metadata = metadata.as_deref().cloned();
                fields.insert("external_metadata".into(), json!(metadata));
            }
        }
        OperationKind::MoveTask {
            section_id,
            position,
            ..
        } => {
            task.section_id = *section_id;
            task.position = position.clone();
            fields.insert("section_id".into(), json!(section_id));
            fields.insert("position".into(), json!(position));
        }
        // the caller only passes a task op; anything else leaves it untouched
        _ => {}
    }

    Value::Object(fields)
}

/// Apply an event update to the row and return the fields the change row
/// records. The caller sets `updated_at` afterwards.
pub(super) fn event_field_updates(event: &mut Event, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    if let OperationKind::UpdateEvent {
        title,
        description,
        interval,
        ..
    } = op
    {
        if let Some(title) = title {
            event.title = title.clone();
            fields.insert("title".into(), json!(title));
        }
        if let Some(description) = description {
            event.description = description.clone();
            fields.insert("description".into(), json!(description));
        }
        if let Some(interval) = interval {
            event.interval = interval.clone();
            fields.insert("interval".into(), json!(interval));
        }
    }

    Value::Object(fields)
}

/// Whether a non-sync update changes a task field that the external metadata
/// marks as externally owned.
///
/// An update that includes `external_metadata` is a sync write and may change
/// those fields. Clearing the identity in the same update also unlocks them.
pub(super) fn update_changes_locked_external_field(task: &Task, op: &OperationKind) -> bool {
    let OperationKind::UpdateTask {
        title,
        description,
        state,
        external_identity,
        external_metadata,
        ..
    } = op
    else {
        return false;
    };

    let has_identity_after = match external_identity {
        Some(identity) => identity.is_some(),
        None => task.external_identity.is_some(),
    };
    if !has_identity_after || external_metadata.is_some() {
        return false;
    }

    let Some(ExternalMetadata::Github {
        title: metadata_title,
        description: metadata_description,
        state: metadata_state,
        ..
    }) = task.external_metadata.as_ref()
    else {
        return false;
    };

    let title_changes = title
        .as_deref()
        .is_some_and(|value| value != task.title.as_str());
    let description_changes = description
        .as_ref()
        .is_some_and(|value| value != &task.description);
    let state_changes = state.as_ref().is_some_and(|value| *value != task.state);

    (metadata_title.is_some() && title_changes)
        || (metadata_description.is_some() && description_changes)
        || (metadata_state.is_some() && state_changes)
}

pub(super) fn project_field_updates(project: &mut Project, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    let OperationKind::UpdateProject {
        name, description, ..
    } = op
    else {
        return Value::Object(fields);
    };

    if let Some(name) = name {
        project.name = name.clone();
        fields.insert("name".into(), json!(name));
    }
    if let Some(description) = description {
        project.description = description.clone();
        fields.insert("description".into(), json!(description));
    }

    Value::Object(fields)
}

pub(super) fn section_field_updates(section: &mut Section, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    match op {
        OperationKind::UpdateSection { name, color, .. } => {
            if let Some(name) = name {
                section.name = name.clone();
                fields.insert("name".into(), json!(name));
            }
            if let Some(color) = color {
                section.color = match color {
                    OperationColorChange::Clear => None,
                    OperationColorChange::Set(color) => Some(*color),
                };
                // a clear records an explicit null so a reader can tell removal from
                // "field unchanged"
                let value = match color {
                    OperationColorChange::Clear => Value::Null,
                    OperationColorChange::Set(color) => json!(color),
                };
                fields.insert("color".into(), value);
            }
        }
        OperationKind::MoveSection { position, .. } => {
            section.position = position.clone();
            fields.insert("position".into(), json!(position));
        }
        _ => {}
    }

    Value::Object(fields)
}

pub(super) fn comment_field_updates(comment: &mut Comment, op: &OperationKind) -> FieldValues {
    let mut fields = Map::new();

    let OperationKind::UpdateComment { contents, .. } = op else {
        return Value::Object(fields);
    };

    comment.contents = contents.clone();
    fields.insert("contents".into(), json!(contents));

    Value::Object(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use scry_core::models::{ExternalIdentity, Priority, Tags, TaskState};
    use scry_core::position::Position;
    use uuid::Uuid;

    fn updates(task: &mut Task, op: &OperationKind) -> FieldValues {
        let updated_at = task.updated_at;
        task_field_updates(task, op, updated_at)
    }

    fn task(title: &str, description: Option<&str>) -> Task {
        let created_at = Timestamp::now();
        Task {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            title: title.to_string(),
            description: description.map(String::from),
            priority: Priority::Normal,
            section_id: None,
            position: Position::parse("a0").expect("a0 is a valid position"),
            state: TaskState::Open,
            closed_reason: None,
            closed_at: None,
            tags: Tags::from(vec!["alpha"]),
            due: None,
            created_at,
            updated_at: created_at,
            external_identity: None,
            external_metadata: None,
        }
    }

    #[test]
    fn update_task_sets_only_the_fields_the_op_carries() {
        let mut task = task("old", Some("kept"));
        let op = OperationKind::UpdateTask {
            id: task.id,
            title: Some("new".into()),
            description: None,
            priority: None,
            tags: None,
            state: None,
            closed_reason: None,
            closed_at: None,
            due: None,
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        };

        let fields = updates(&mut task, &op);

        assert_eq!(task.title, "new");
        assert_eq!(task.description.as_deref(), Some("kept"));

        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields["title"], json!("new"));
        assert!(!fields.contains_key("description"));
    }

    #[test]
    fn later_update_replaces_the_earlier_value() {
        let mut task = task("old", None);
        let id = task.id;

        updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: Some("first".into()),
                description: None,
                priority: None,
                tags: None,
                state: None,
                closed_reason: None,
                closed_at: None,
                due: None,
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );
        let fields = updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: Some("second".into()),
                description: None,
                priority: None,
                tags: None,
                state: None,
                closed_reason: None,
                closed_at: None,
                due: None,
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );

        assert_eq!(task.title, "second");
        assert_eq!(fields["title"], json!("second"));
    }

    fn github_metadata() -> ExternalMetadata {
        ExternalMetadata::Github {
            title: Some("Fix login".to_string()),
            description: Some("Users cannot sign in".to_string()),
            kind: Some("issue".to_string()),
            state: Some("open".to_string()),
            reason: Some("reopened".to_string()),
            draft: Some(false),
            updated_at: Some(Timestamp::now()),
            synced_at: None,
            error: None,
        }
    }

    fn github_metadata_with(
        title: Option<&str>,
        description: Option<&str>,
        state: Option<&str>,
    ) -> ExternalMetadata {
        ExternalMetadata::Github {
            title: title.map(str::to_string),
            description: description.map(str::to_string),
            kind: Some("issue".to_string()),
            state: state.map(str::to_string),
            reason: None,
            draft: Some(false),
            updated_at: None,
            synced_at: None,
            error: None,
        }
    }

    fn linked_task(metadata: ExternalMetadata) -> Task {
        let mut task = task("linked", Some("external description"));
        task.external_identity = Some(ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 42,
        });
        task.external_metadata = Some(metadata);
        task
    }

    fn lock_op(
        task: &Task,
        title: Option<&str>,
        description: Option<Option<&str>>,
        state: Option<TaskState>,
        external_identity: Option<Option<ExternalIdentity>>,
        external_metadata: Option<Option<Box<ExternalMetadata>>>,
    ) -> OperationKind {
        OperationKind::UpdateTask {
            id: task.id,
            title: title.map(str::to_string),
            description: description.map(|description| description.map(str::to_string)),
            priority: None,
            tags: None,
            state,
            closed_reason: None,
            closed_at: None,
            due: None,
            external_identity,
            external_metadata,
            updated_at: None,
        }
    }

    #[test]
    fn update_task_sets_and_clears_external_fields() {
        let mut task = task("t", None);
        let id = task.id;
        let identity = ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 42,
        };
        let metadata = github_metadata();

        let fields = updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: None,
                closed_reason: None,
                closed_at: None,
                due: None,
                external_identity: Some(Some(identity.clone())),
                external_metadata: Some(Some(Box::new(metadata.clone()))),
                updated_at: None,
            },
        );

        assert_eq!(task.external_identity, Some(identity.clone()));
        assert_eq!(task.external_metadata, Some(metadata.clone()));
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["external_identity"], json!(identity));
        assert_eq!(fields["external_metadata"], json!(metadata));

        let fields = updates(
            &mut task,
            &OperationKind::UpdateTask {
                id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: None,
                closed_reason: None,
                closed_at: None,
                due: None,
                external_identity: Some(None),
                external_metadata: Some(None),
                updated_at: None,
            },
        );

        assert_eq!(task.external_identity, None);
        assert_eq!(task.external_metadata, None);
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["external_identity"], json!(null));
        assert_eq!(fields["external_metadata"], json!(null));
    }

    #[test]
    fn linked_task_blocks_locked_title_change() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("open"),
        ));
        let op = lock_op(&task, Some("New title"), None, None, None, None);

        assert!(update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn sync_update_may_change_locked_title() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("open"),
        ));
        let op = lock_op(
            &task,
            Some("New title"),
            None,
            None,
            None,
            Some(Some(Box::new(github_metadata_with(
                Some("Fix login"),
                Some("Users cannot sign in"),
                Some("open"),
            )))),
        );

        assert!(!update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn metadata_without_title_allows_title_change() {
        let task = linked_task(github_metadata_with(None, None, None));
        let op = lock_op(&task, Some("New title"), None, None, None, None);

        assert!(!update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn linked_task_blocks_a_real_description_change_only() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("open"),
        ));

        let changed = lock_op(
            &task,
            None,
            Some(Some("A new description")),
            None,
            None,
            None,
        );
        assert!(update_changes_locked_external_field(&task, &changed));

        let unchanged = lock_op(
            &task,
            None,
            Some(Some("external description")),
            None,
            None,
            None,
        );
        assert!(!update_changes_locked_external_field(&task, &unchanged));
    }

    #[test]
    fn linked_task_blocks_locked_state_change() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("closed"),
        ));
        let op = lock_op(&task, None, None, Some(TaskState::Closed), None, None);

        assert!(update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn priority_and_tags_are_not_locked() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("open"),
        ));
        let mut op = lock_op(&task, None, None, None, None, None);
        if let OperationKind::UpdateTask { priority, tags, .. } = &mut op {
            *priority = Some(Priority::Elevated);
            *tags = Some(Tags::from(vec!["urgent"]));
        }

        assert!(!update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn clearing_identity_unlocks_title_change() {
        let task = linked_task(github_metadata_with(
            Some("Fix login"),
            Some("Users cannot sign in"),
            Some("open"),
        ));
        let op = lock_op(&task, Some("New title"), None, None, Some(None), None);

        assert!(!update_changes_locked_external_field(&task, &op));
    }

    #[test]
    fn unlinked_task_is_fully_editable() {
        let task = task("plain", Some("a description"));
        let op = lock_op(
            &task,
            Some("New title"),
            Some(Some("A new description")),
            Some(TaskState::Closed),
            None,
            None,
        );

        assert!(!update_changes_locked_external_field(&task, &op));
    }

    fn close_op(
        id: Uuid,
        state: TaskState,
        closed_reason: Option<ClosedReason>,
        closed_at: Option<Timestamp>,
    ) -> OperationKind {
        OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: Some(state),
            closed_reason: closed_reason.map(Some),
            closed_at: closed_at.map(Some),
            due: None,
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        }
    }

    #[test]
    fn closing_without_a_reason_records_completed() {
        let mut task = task("t", None);
        let id = task.id;

        let fields = updates(&mut task, &close_op(id, TaskState::Closed, None, None));

        assert_eq!(task.closed_reason, Some(ClosedReason::Completed));
        assert_eq!(task.closed_at, Some(task.updated_at));
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["closed_reason"], json!(ClosedReason::Completed));
        assert_eq!(fields["closed_at"], json!(task.updated_at));
    }

    #[test]
    fn closing_records_an_explicit_reason() {
        let mut task = task("t", None);
        let id = task.id;

        let fields = updates(
            &mut task,
            &close_op(id, TaskState::Closed, Some(ClosedReason::Skipped), None),
        );

        assert_eq!(task.closed_reason, Some(ClosedReason::Skipped));
        assert_eq!(task.closed_at, Some(task.updated_at));
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["closed_reason"], json!(ClosedReason::Skipped));
    }

    #[test]
    fn reopening_clears_close_metadata() {
        let mut task = task("t", None);
        task.state = TaskState::Closed;
        task.closed_reason = Some(ClosedReason::Cancelled);
        task.closed_at = Some(task.updated_at);
        let id = task.id;

        let fields = updates(&mut task, &close_op(id, TaskState::Open, None, None));

        assert_eq!(task.state, TaskState::Open);
        assert_eq!(task.closed_reason, None);
        assert_eq!(task.closed_at, None);
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["closed_reason"], json!(null));
        assert_eq!(fields["closed_at"], json!(null));
    }

    #[test]
    fn close_fields_are_ignored_on_an_open_task() {
        let mut task = task("t", None);
        let id = task.id;
        let op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: None,
            closed_reason: Some(Some(ClosedReason::Cancelled)),
            closed_at: Some(Some(Timestamp::now())),
            due: None,
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        };

        let fields = updates(&mut task, &op);

        assert_eq!(task.state, TaskState::Open);
        assert_eq!(task.closed_reason, None);
        assert_eq!(task.closed_at, None);
        let fields = fields.as_object().expect("the field map is an object");
        assert!(!fields.contains_key("closed_reason"));
        assert!(!fields.contains_key("closed_at"));
    }

    #[test]
    fn explicit_close_fields_apply_while_closed() {
        let mut task = task("t", None);
        task.state = TaskState::Closed;
        task.closed_reason = Some(ClosedReason::Completed);
        task.closed_at = Some(task.updated_at);
        let id = task.id;
        let at = Timestamp::now();
        let op = OperationKind::UpdateTask {
            id,
            title: None,
            description: None,
            priority: None,
            tags: None,
            state: None,
            closed_reason: Some(Some(ClosedReason::Skipped)),
            closed_at: Some(Some(at)),
            due: None,
            external_identity: None,
            external_metadata: None,
            updated_at: None,
        };

        let fields = updates(&mut task, &op);

        assert_eq!(task.closed_reason, Some(ClosedReason::Skipped));
        assert_eq!(task.closed_at, Some(at));
        let fields = fields.as_object().expect("the field map is an object");
        assert_eq!(fields["closed_reason"], json!(ClosedReason::Skipped));
        assert_eq!(fields["closed_at"], json!(at));
    }
}
