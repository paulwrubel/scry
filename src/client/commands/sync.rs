//! `sync`.

use std::process::Command;

use chrono::{DateTime, Utc};
use scry_core::github::{placeholder_title, should_apply_metadata, task_state_for_external_state};
use scry_core::ids::TaskId;
use scry_core::models::{ExternalIdentity, ExternalMetadata, Task, TaskState};
use scry_core::snapshot::Snapshot;
use serde::Serialize;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

const GITHUB_QUERY: &str = r#"query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    issueOrPullRequest(number: $number) {
      __typename
      ... on Issue {
        title
        body
        state
        stateReason
        updatedAt
      }
      ... on PullRequest {
        title
        body
        state
        isDraft
        updatedAt
      }
    }
  }
}"#;

/// The external system a task can be synced with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Provider {
    Github,
}

/// Sync tasks that link to an external issue or pull request.
pub async fn sync(
    client: &Client,
    provider: Option<Provider>,
    output_json: bool,
) -> Result<(), AppError> {
    let snapshot: Snapshot = client.get_json(&["snapshot"], &[]).await?;

    let mut outcomes = Vec::new();
    for project in &snapshot.projects {
        for task in &project.tasks {
            let Some(identity) = &task.external_identity else {
                continue;
            };
            if !provider_matches(provider, identity) {
                continue;
            }
            outcomes.push(sync_task(client, task, identity).await);
        }
    }

    if output_json {
        print_json(&outcomes)?;
        return Ok(());
    }

    for outcome in &outcomes {
        let label = placeholder_title(&outcome.identity);
        match outcome.status {
            "synced" => println!("{label}: synced"),
            "skipped" => println!("{label}: up to date"),
            _ => {
                let error = outcome.error.as_deref().unwrap_or("error");
                match &outcome.detail {
                    Some(detail) => println!("{label}: error: {error} ({detail})"),
                    None => println!("{label}: error: {error}"),
                }
            }
        }
    }

    Ok(())
}

/// Why a GitHub read could not produce usable metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncError {
    GhFailed,
    NotFound,
    InvalidResponse,
}

impl SyncError {
    fn code(self) -> &'static str {
        match self {
            Self::GhFailed => "gh_failed",
            Self::NotFound => "not_found",
            Self::InvalidResponse => "invalid_response",
        }
    }
}

/// The subset of GitHub issue or pull request fields scry mirrors.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GithubState {
    title: String,
    description: Option<String>,
    kind: String,
    state: String,
    reason: Option<String>,
    draft: Option<bool>,
    updated_at: DateTime<Utc>,
}

/// A task update containing only the fields that changed.
#[derive(Serialize)]
struct SyncUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<TaskState>,
    external_metadata: Option<Option<ExternalMetadata>>,
}

/// The per-task result reported by `sync`.
#[derive(Serialize)]
struct SyncOutcome {
    task_id: TaskId,
    identity: ExternalIdentity,
    status: &'static str,
    error: Option<String>,
    detail: Option<String>,
}

async fn sync_task(client: &Client, task: &Task, identity: &ExternalIdentity) -> SyncOutcome {
    let ExternalIdentity::Github {
        owner,
        repo,
        number,
    } = identity;

    let state = match fetch_state(owner, repo, *number) {
        Ok(state) => state,
        Err(error) => return record_error(client, task, identity, error).await,
    };

    if !should_apply_task_metadata(task.external_metadata.as_ref(), state.updated_at) {
        return SyncOutcome {
            task_id: task.id,
            identity: identity.clone(),
            status: "skipped",
            error: None,
            detail: None,
        };
    }

    let metadata = github_metadata(&state);
    let update = match build_success_update(task, &state, metadata) {
        Ok(update) => update,
        Err(error) => return record_error(client, task, identity, error).await,
    };

    match put_task(client, task, &update).await {
        Ok(()) => SyncOutcome {
            task_id: task.id,
            identity: identity.clone(),
            status: "synced",
            error: None,
            detail: None,
        },
        Err(error) => SyncOutcome {
            task_id: task.id,
            identity: identity.clone(),
            status: "error",
            error: Some(error.kind().to_string()),
            detail: Some(error.to_string()),
        },
    }
}

/// Record a failed observation without changing any task content.
async fn record_error(
    client: &Client,
    task: &Task,
    identity: &ExternalIdentity,
    error: SyncError,
) -> SyncOutcome {
    let metadata = error_metadata(task.external_metadata.as_ref(), error);
    let update = SyncUpdate {
        title: None,
        description: None,
        state: None,
        external_metadata: Some(Some(metadata)),
    };

    let detail = match put_task(client, task, &update).await {
        Ok(()) => None,
        Err(record_error) => Some(format!("failed to record error: {record_error}")),
    };

    SyncOutcome {
        task_id: task.id,
        identity: identity.clone(),
        status: "error",
        error: Some(error.code().to_string()),
        detail,
    }
}

fn provider_matches(provider: Option<Provider>, identity: &ExternalIdentity) -> bool {
    match provider {
        None => true,
        Some(Provider::Github) => matches!(identity, ExternalIdentity::Github { .. }),
    }
}

/// Whether an observation should overwrite the stored metadata.
///
/// A stored error wins over the timestamp guard so the next successful read can
/// clear it, even when GitHub's `updatedAt` has not moved.
fn should_apply_task_metadata(
    existing: Option<&ExternalMetadata>,
    updated_at: DateTime<Utc>,
) -> bool {
    has_error(existing) || should_apply_metadata(existing, updated_at)
}

fn has_error(metadata: Option<&ExternalMetadata>) -> bool {
    matches!(
        metadata,
        Some(ExternalMetadata::Github { error: Some(_), .. })
    )
}

fn github_metadata(state: &GithubState) -> ExternalMetadata {
    ExternalMetadata::Github {
        title: Some(state.title.clone()),
        description: state.description.clone(),
        kind: Some(state.kind.clone()),
        state: Some(state.state.clone()),
        reason: state.reason.clone(),
        draft: state.draft,
        updated_at: Some(state.updated_at),
        error: None,
    }
}

fn error_metadata(existing: Option<&ExternalMetadata>, error: SyncError) -> ExternalMetadata {
    match existing {
        Some(ExternalMetadata::Github {
            title,
            description,
            kind,
            state,
            reason,
            draft,
            updated_at,
            ..
        }) => ExternalMetadata::Github {
            title: title.clone(),
            description: description.clone(),
            kind: kind.clone(),
            state: state.clone(),
            reason: reason.clone(),
            draft: *draft,
            updated_at: *updated_at,
            error: Some(error.code().to_string()),
        },
        None => ExternalMetadata::Github {
            title: None,
            description: None,
            kind: None,
            state: None,
            reason: None,
            draft: None,
            updated_at: None,
            error: Some(error.code().to_string()),
        },
    }
}

fn build_success_update(
    task: &Task,
    state: &GithubState,
    metadata: ExternalMetadata,
) -> Result<SyncUpdate, SyncError> {
    let task_state =
        task_state_for_external_state(&state.state).ok_or(SyncError::InvalidResponse)?;

    let title = if state.title != task.title {
        Some(state.title.clone())
    } else {
        None
    };
    let description = match state.description.as_deref() {
        Some(description) if task.description.as_deref() != Some(description) => {
            Some(description.to_string())
        }
        // an empty string is the API's way to clear a description
        None if task.description.is_some() => Some(String::new()),
        _ => None,
    };
    let state_field = (task_state != task.state).then_some(task_state);

    Ok(SyncUpdate {
        title,
        description,
        state: state_field,
        external_metadata: Some(Some(metadata)),
    })
}

async fn put_task(client: &Client, task: &Task, update: &SyncUpdate) -> Result<(), AppError> {
    let task_id = task.id.to_string();
    let _: Task = client
        .put_json(&["tasks", task_id.as_str()], &[], update)
        .await?;
    Ok(())
}

fn fetch_state(owner: &str, repo: &str, number: u64) -> Result<GithubState, SyncError> {
    let output = Command::new("gh")
        .arg("api")
        .arg("graphql")
        .arg("-f")
        .arg(format!("query={GITHUB_QUERY}"))
        .arg("-f")
        .arg(format!("owner={owner}"))
        .arg("-f")
        .arg(format!("name={repo}"))
        .arg("-F")
        .arg(format!("number={number}"))
        .output()
        .map_err(|_| SyncError::GhFailed)?;

    if !output.status.success() {
        return Err(SyncError::GhFailed);
    }

    let body = String::from_utf8_lossy(&output.stdout);
    parse_github_state(&body)
}

fn parse_github_state(body: &str) -> Result<GithubState, SyncError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| SyncError::InvalidResponse)?;
    parse_github_node(&value)
}

fn parse_github_node(value: &serde_json::Value) -> Result<GithubState, SyncError> {
    let node = value
        .get("data")
        .and_then(|data| data.get("repository"))
        .and_then(|repository| repository.get("issueOrPullRequest"))
        .ok_or(SyncError::InvalidResponse)?;

    if node.is_null() {
        return Err(SyncError::NotFound);
    }

    let typename = node
        .get("__typename")
        .and_then(serde_json::Value::as_str)
        .ok_or(SyncError::InvalidResponse)?;
    let title = node
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or(SyncError::InvalidResponse)?
        .to_string();
    let state = node
        .get("state")
        .and_then(serde_json::Value::as_str)
        .ok_or(SyncError::InvalidResponse)?
        .to_lowercase();
    let updated_at = node
        .get("updatedAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|parsed| parsed.with_timezone(&Utc))
        .ok_or(SyncError::InvalidResponse)?;
    let description = node
        .get("body")
        .and_then(serde_json::Value::as_str)
        .filter(|body| !body.is_empty())
        .map(str::to_string);

    match typename {
        "Issue" => {
            let reason = node
                .get("stateReason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_lowercase);
            Ok(GithubState {
                title,
                description,
                kind: "issue".to_string(),
                state,
                reason,
                draft: None,
                updated_at,
            })
        }
        "PullRequest" => Ok(GithubState {
            title,
            description,
            kind: "pull".to_string(),
            state,
            reason: None,
            draft: node.get("isDraft").and_then(serde_json::Value::as_bool),
            updated_at,
        }),
        _ => Err(SyncError::InvalidResponse),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scry_core::models::Priority;
    use scry_core::models::Tags;
    use scry_core::position::Position;
    use uuid::Uuid;

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn response(node: serde_json::Value) -> String {
        serde_json::json!({
            "data": {
                "repository": {
                    "issueOrPullRequest": node,
                },
            },
        })
        .to_string()
    }

    fn issue_node(body: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "__typename": "Issue",
            "title": "Bug",
            "body": body,
            "state": "CLOSED",
            "stateReason": "COMPLETED",
            "updatedAt": "2024-05-01T12:00:00Z",
        })
    }

    fn task(metadata: Option<ExternalMetadata>) -> Task {
        Task {
            id: Uuid::nil(),
            project_id: Uuid::nil(),
            title: "placeholder".to_string(),
            description: None,
            priority: Priority::default(),
            section_id: None,
            position: Position::parse("a0").unwrap(),
            state: TaskState::Open,
            tags: Tags::default(),
            created_at: timestamp("2024-01-01T00:00:00Z"),
            updated_at: timestamp("2024-01-01T00:00:00Z"),
            external_identity: None,
            external_metadata: metadata,
        }
    }

    fn stored_metadata(updated_at: Option<DateTime<Utc>>, error: Option<&str>) -> ExternalMetadata {
        ExternalMetadata::Github {
            title: Some("Previous".to_string()),
            description: Some("old body".to_string()),
            kind: Some("issue".to_string()),
            state: Some("open".to_string()),
            reason: Some("reopened".to_string()),
            draft: None,
            updated_at,
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn parses_closed_issue() {
        let state = parse_github_state(&response(issue_node(serde_json::json!("A body")))).unwrap();

        assert_eq!(state.title, "Bug");
        assert_eq!(state.description.as_deref(), Some("A body"));
        assert_eq!(state.kind, "issue");
        assert_eq!(state.state, "closed");
        assert_eq!(state.reason.as_deref(), Some("completed"));
        assert_eq!(state.draft, None);
        assert_eq!(state.updated_at, timestamp("2024-05-01T12:00:00Z"));
    }

    #[test]
    fn parses_merged_draft_pull_request() {
        let node = serde_json::json!({
            "__typename": "PullRequest",
            "title": "Feature",
            "body": "A body",
            "state": "MERGED",
            "isDraft": true,
            "updatedAt": "2024-06-01T00:00:00Z",
        });
        let state = parse_github_state(&response(node)).unwrap();

        assert_eq!(state.title, "Feature");
        assert_eq!(state.kind, "pull");
        assert_eq!(state.state, "merged");
        assert_eq!(state.reason, None);
        assert_eq!(state.draft, Some(true));
        assert_eq!(state.updated_at, timestamp("2024-06-01T00:00:00Z"));
    }

    #[test]
    fn empty_body_maps_to_none() {
        let empty = parse_github_state(&response(issue_node(serde_json::json!("")))).unwrap();
        assert_eq!(empty.description, None);

        let null = parse_github_state(&response(issue_node(serde_json::Value::Null))).unwrap();
        assert_eq!(null.description, None);
    }

    #[test]
    fn null_issue_or_pull_request_is_not_found() {
        let error = parse_github_state(&response(serde_json::Value::Null)).unwrap_err();
        assert_eq!(error, SyncError::NotFound);
        assert_eq!(error.code(), "not_found");
    }

    #[test]
    fn unknown_typename_is_invalid_response() {
        let node = serde_json::json!({
            "__typename": "Discussion",
            "title": "Something",
            "body": "",
            "state": "OPEN",
            "updatedAt": "2024-05-01T12:00:00Z",
        });
        let error = parse_github_state(&response(node)).unwrap_err();
        assert_eq!(error, SyncError::InvalidResponse);
        assert_eq!(error.code(), "invalid_response");
    }

    #[test]
    fn error_metadata_preserves_previous_fields() {
        let existing = stored_metadata(Some(timestamp("2024-05-01T12:00:00Z")), None);
        let metadata = error_metadata(Some(&existing), SyncError::NotFound);

        match metadata {
            ExternalMetadata::Github {
                title,
                description,
                kind,
                state,
                reason,
                draft,
                updated_at,
                error,
            } => {
                assert_eq!(title.as_deref(), Some("Previous"));
                assert_eq!(description.as_deref(), Some("old body"));
                assert_eq!(kind.as_deref(), Some("issue"));
                assert_eq!(state.as_deref(), Some("open"));
                assert_eq!(reason.as_deref(), Some("reopened"));
                assert_eq!(draft, None);
                assert_eq!(updated_at, Some(timestamp("2024-05-01T12:00:00Z")));
                assert_eq!(error.as_deref(), Some("not_found"));
            }
        }
    }

    #[test]
    fn stored_error_overrides_stale_timestamp() {
        let updated_at = timestamp("2024-05-01T12:00:00Z");

        let errored = stored_metadata(Some(updated_at), Some("gh_failed"));
        assert!(should_apply_task_metadata(Some(&errored), updated_at));

        let clean = stored_metadata(Some(updated_at), None);
        assert!(!should_apply_task_metadata(Some(&clean), updated_at));
    }

    #[test]
    fn update_sends_only_changed_fields() {
        let state = GithubState {
            title: "placeholder".to_string(),
            description: None,
            kind: "issue".to_string(),
            state: "closed".to_string(),
            reason: None,
            draft: None,
            updated_at: timestamp("2024-05-01T12:00:00Z"),
        };
        let metadata = github_metadata(&state);
        let update = build_success_update(&task(None), &state, metadata).unwrap();

        assert_eq!(update.title, None);
        assert_eq!(update.description, None);
        assert_eq!(update.state, Some(TaskState::Closed));
        assert!(update.external_metadata.is_some());
    }

    #[test]
    fn update_clears_missing_description() {
        let state = GithubState {
            title: "placeholder".to_string(),
            description: None,
            kind: "issue".to_string(),
            state: "open".to_string(),
            reason: None,
            draft: None,
            updated_at: timestamp("2024-05-01T12:00:00Z"),
        };
        let mut existing = task(None);
        existing.description = Some("stale".to_string());
        let metadata = github_metadata(&state);
        let update = build_success_update(&existing, &state, metadata).unwrap();

        assert_eq!(update.description.as_deref(), Some(""));
    }
}
