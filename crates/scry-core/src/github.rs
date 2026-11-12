use crate::models::{ExternalIdentity, ExternalMetadata, TaskState};
use chrono::{DateTime, Utc};
use std::fmt;

/// Why a string could not be parsed as a GitHub issue or pull request URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubIdentityError {
    /// The input is not an `https://github.com/...` URL.
    NotGithubUrl,
    /// The path does not have the shape `{owner}/{repo}/{issues|pull}/...`.
    MalformedPath,
    /// The trailing number is not a valid unsigned integer.
    InvalidNumber(String),
    /// GitHub issue and pull numbers start at 1.
    ZeroNumber,
    /// The owner or repo segment is empty or contains whitespace.
    InvalidName,
}

impl fmt::Display for GithubIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotGithubUrl => {
                f.write_str("only https://github.com issue and pull URLs are supported")
            }
            Self::MalformedPath => {
                f.write_str("expected https://github.com/{owner}/{repo}/{issues|pull}/{number}")
            }
            Self::InvalidNumber(value) => write!(f, "invalid GitHub number: {value:?}"),
            Self::ZeroNumber => f.write_str("GitHub issue and pull numbers start at 1"),
            Self::InvalidName => {
                f.write_str("GitHub owner and repo names cannot be empty or contain whitespace")
            }
        }
    }
}

impl std::error::Error for GithubIdentityError {}

/// Parse a GitHub issue or pull request URL into an [`ExternalIdentity`].
///
/// Accepts `https://github.com/{owner}/{repo}/issues/{number}` and
/// `https://github.com/{owner}/{repo}/pull/{number}`, ignoring any subpath,
/// trailing slash, query string, or fragment. Owner and repo are lowercased so
/// the same repository always produces one identity.
pub fn parse_github_identity(input: &str) -> Result<ExternalIdentity, GithubIdentityError> {
    // strip the fragment before the query so a '?' inside the fragment cannot
    // be mistaken for the start of the query string
    let without_fragment = input.split_once('#').map_or(input, |(head, _)| head);
    let without_query = without_fragment
        .split_once('?')
        .map_or(without_fragment, |(head, _)| head);

    let path = without_query
        .strip_prefix("https://github.com/")
        .ok_or(GithubIdentityError::NotGithubUrl)?;

    let mut segments = path.split('/');
    let owner = segments.next().unwrap_or("");
    let repo = segments.next().unwrap_or("");
    let kind = segments.next().unwrap_or("");
    let number = segments.next().unwrap_or("");

    if kind != "issues" && kind != "pull" {
        return Err(GithubIdentityError::MalformedPath);
    }

    if !is_valid_name(owner) || !is_valid_name(repo) {
        return Err(GithubIdentityError::InvalidName);
    }

    let number = number
        .parse::<u64>()
        .map_err(|_| GithubIdentityError::InvalidNumber(number.to_string()))?;
    if number == 0 {
        return Err(GithubIdentityError::ZeroNumber);
    }

    Ok(ExternalIdentity::Github {
        owner: owner.to_lowercase(),
        repo: repo.to_lowercase(),
        number,
    })
}

/// A human-readable title to use until real metadata is fetched.
pub fn placeholder_title(identity: &ExternalIdentity) -> String {
    match identity {
        ExternalIdentity::Github {
            owner,
            repo,
            number,
        } => format!("{owner}/{repo}#{number}"),
    }
}

/// Map a GitHub state string onto a task state.
///
/// GitHub reports `merged` as a separate value for pulls, but a merged pull is
/// still closed as far as a task is concerned.
pub fn task_state_for_external_state(state: &str) -> Option<TaskState> {
    match state {
        "open" => Some(TaskState::Open),
        "closed" | "merged" => Some(TaskState::Closed),
        _ => None,
    }
}

/// Whether fetched metadata is newer than what is already stored.
///
/// A missing timestamp on either side means we cannot prove the stored copy is
/// current, so the incoming metadata wins.
pub fn should_apply_metadata(
    existing: Option<&ExternalMetadata>,
    updated_at: DateTime<Utc>,
) -> bool {
    match existing {
        Some(ExternalMetadata::Github {
            updated_at: Some(existing_updated_at),
            ..
        }) => updated_at > *existing_updated_at,
        _ => true,
    }
}

fn is_valid_name(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn acme_widgets() -> ExternalIdentity {
        ExternalIdentity::Github {
            owner: "acme".to_string(),
            repo: "widgets".to_string(),
            number: 123,
        }
    }

    fn github_metadata(updated_at: Option<DateTime<Utc>>) -> ExternalMetadata {
        ExternalMetadata::Github {
            title: None,
            description: None,
            kind: None,
            state: None,
            reason: None,
            draft: None,
            updated_at,
            error: None,
        }
    }

    #[test]
    fn parses_accepted_urls() {
        let urls = [
            "https://github.com/acme/widgets/issues/123",
            "https://github.com/acme/widgets/pull/123",
            "https://github.com/acme/widgets/pull/123/files",
            "https://github.com/acme/widgets/issues/123/",
            "https://github.com/acme/widgets/issues/123?foo=bar",
            "https://github.com/acme/widgets/issues/123#discussion",
            "https://github.com/ACME/Widgets/issues/123",
        ];

        for url in urls {
            assert_eq!(
                parse_github_identity(url).unwrap(),
                acme_widgets(),
                "url {url}"
            );
        }
    }

    #[test]
    fn rejects_invalid_urls() {
        let urls = [
            "https://gitlab.com/acme/widgets/issues/123",
            "http://github.com/acme/widgets/issues/123",
            "https://github.com/acme/issues/123",
            "https://github.com/acme/widgets/blob/main/README.md",
            "https://github.com/acme/widgets/issues/abc",
            "https://github.com/acme/widgets/issues/0",
            "https://github.com/ac me/widgets/issues/123",
            "",
        ];

        for url in urls {
            assert!(
                parse_github_identity(url).is_err(),
                "url {url} should have been rejected"
            );
        }
    }

    #[test]
    fn placeholder_title_is_owner_repo_number() {
        assert_eq!(placeholder_title(&acme_widgets()), "acme/widgets#123");
    }

    #[test]
    fn maps_github_states_to_task_states() {
        assert_eq!(task_state_for_external_state("open"), Some(TaskState::Open));
        assert_eq!(
            task_state_for_external_state("closed"),
            Some(TaskState::Closed)
        );
        assert_eq!(
            task_state_for_external_state("merged"),
            Some(TaskState::Closed)
        );
        assert_eq!(task_state_for_external_state("draft"), None);
    }

    #[test]
    fn applies_metadata_only_when_strictly_newer() {
        let earlier = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 1).unwrap();

        assert!(should_apply_metadata(None, earlier));

        let no_timestamp = github_metadata(None);
        assert!(should_apply_metadata(Some(&no_timestamp), earlier));

        let stored = github_metadata(Some(earlier));
        assert!(should_apply_metadata(Some(&stored), later));
        assert!(!should_apply_metadata(Some(&stored), earlier));
        assert!(!should_apply_metadata(
            Some(&stored),
            Utc.with_ymd_and_hms(2023, 12, 31, 23, 59, 59).unwrap()
        ));
    }
}
