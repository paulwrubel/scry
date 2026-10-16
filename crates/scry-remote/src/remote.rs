//! [`RemoteStore`]: a [`Store`] backed by scry's HTTP REST API.

use async_trait::async_trait;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use scry_core::backup::{ImportPlan, ImportReport};
use scry_core::error::StorageError;
use scry_core::models::{
    Color, Note, NoteId, Priority, Project, ProjectId, Status, StatusId, StatusStyle, Task, TaskId,
    TaskSortingMode,
};
use scry_core::store::{Store, TaskToCreate};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A [`Store`] that drives a remote scry server over HTTP.
pub struct RemoteStore {
    client: Client,
    api_url: String,
    token: Option<String>,
}

/// The REST API's flat error body: `{"code", "kind", "message"}`.
///
/// Only `kind` and `message` are needed; the redundant `code` field is ignored
/// by serde, matching the `impl From<AppError> for Response` in the server.
#[derive(Deserialize)]
struct ApiError {
    kind: String,
    message: String,
}

/// Body for `POST /projects/{id}/tasks`; `project_id` travels in the path.
#[derive(Serialize)]
struct CreateTaskBody {
    title: String,
    description: Option<String>,
    priority: Priority,
    status_id: i64,
    position: i32,
    tags: Vec<String>,
}

/// Body for `POST /projects/{id}/statuses`; `project_id` travels in the path.
#[derive(Serialize)]
struct CreateStatusBody {
    name: String,
    position: i32,
    color: Option<Color>,
    style: StatusStyle,
}

/// Body for `POST /tasks/{id}/notes`; the task id travels in the path.
#[derive(Serialize)]
struct CreateNoteBody {
    contents: String,
}

/// Body for `POST /projects`.
#[derive(Serialize)]
struct CreateProjectBody {
    name: String,
    entry_status_id: Option<StatusId>,
    task_sorting_mode: TaskSortingMode,
    show_priority: bool,
}

/// Body for `POST /statuses/{id}/move`.
#[derive(Serialize)]
struct MoveStatusBody {
    position: i32,
}

impl RemoteStore {
    /// Build a client for the API rooted at `base_url` (e.g. `http://host:8080`).
    ///
    /// A trailing slash on `base_url` is trimmed before the `/api/v1` prefix is
    /// appended. `token`, when present, is sent as every request's bearer token.
    pub async fn new(base_url: &str, token: Option<String>) -> Result<Self, StorageError> {
        let trimmed = base_url.trim_end_matches('/');
        let api_url = format!("{trimmed}/api/v1");
        let client = Client::builder().build().map_err(|error| {
            StorageError::Database(format!("failed to build HTTP client: {error}"))
        })?;

        Ok(Self {
            client,
            api_url,
            token,
        })
    }

    /// Resolve a request URL by appending percent-encoded path `segments`.
    ///
    /// Each segment is encoded independently, so names containing spaces or
    /// slashes survive the round trip.
    fn url(&self, segments: &[&str]) -> Result<Url, StorageError> {
        let mut url = Url::parse(&self.api_url)
            .map_err(|error| StorageError::Database(format!("invalid API URL: {error}")))?;
        url.path_segments_mut()
            .map_err(|_| StorageError::Database("API URL cannot be a base".to_string()))?
            .extend(segments.iter().copied());
        Ok(url)
    }

    /// Start a request, attaching the configured bearer token when present.
    fn request(&self, method: Method, segments: &[&str]) -> Result<RequestBuilder, StorageError> {
        let mut builder = self.client.request(method, self.url(segments)?);
        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token);
        }
        Ok(builder)
    }

    /// Send a request, mapping transport failures and non-2xx responses to errors.
    async fn send(&self, builder: RequestBuilder) -> Result<Response, StorageError> {
        let response = builder
            .send()
            .await
            .map_err(|error| StorageError::Database(format!("HTTP request failed: {error}")))?;

        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_default();
        Err(error_from_response(status, &body))
    }

    /// Send a request and decode its JSON body.
    async fn send_json<T: DeserializeOwned + Send>(
        &self,
        builder: RequestBuilder,
    ) -> Result<T, StorageError> {
        let response = self.send(builder).await?;
        response.json::<T>().await.map_err(|error| {
            StorageError::Database(format!("failed to decode response body: {error}"))
        })
    }

    /// Send a request whose success response carries no body (`204 No Content`).
    async fn send_empty(&self, builder: RequestBuilder) -> Result<(), StorageError> {
        self.send(builder).await?;
        Ok(())
    }
}

/// Translate a non-2xx response into a [`StorageError`], honoring the error `kind`.
///
/// The REST API's flat error body is `{"code", "kind", "message"}`; an
/// unparseable body is reported verbatim.
fn error_from_response(status: StatusCode, body: &str) -> StorageError {
    let Ok(error) = serde_json::from_str::<ApiError>(body) else {
        return StorageError::Database(format!("HTTP {status}: {body}"));
    };

    match error.kind.as_str() {
        "not_found" => StorageError::NotFound(error.message),
        "conflict" => StorageError::Conflict(error.message),
        "invalid" => StorageError::Invalid(error.message),
        _ => StorageError::Database(error.message),
    }
}

#[async_trait]
impl Store for RemoteStore {
    async fn create_task(&self, task_to_create: TaskToCreate) -> Result<Task, StorageError> {
        let project_id = task_to_create.project_id.to_string();
        let body = CreateTaskBody {
            title: task_to_create.title,
            description: task_to_create.description,
            priority: task_to_create.priority,
            status_id: task_to_create.status_id,
            position: task_to_create.position,
            tags: Vec::<String>::from(task_to_create.tags),
        };
        let builder = self
            .request(Method::POST, &["projects", &project_id, "tasks"])?
            .json(&body);
        self.send_json(builder).await
    }

    async fn get_task_by_id(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::GET, &["tasks", &id])?;
        self.send_json(builder).await
    }

    async fn get_all_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, StorageError> {
        let project_id = project_id.to_string();
        let builder = self.request(Method::GET, &["projects", &project_id, "tasks"])?;
        self.send_json(builder).await
    }

    async fn get_all_tasks_by_status_id(
        &self,
        status_id: StatusId,
    ) -> Result<Vec<Task>, StorageError> {
        let status_id = status_id.to_string();
        let builder = self.request(Method::GET, &["statuses", &status_id, "tasks"])?;
        self.send_json(builder).await
    }

    async fn update_task(&self, task: Task) -> Result<Task, StorageError> {
        let id = task.id.to_string();
        let builder = self.request(Method::PUT, &["tasks", &id])?.json(&task);
        self.send_json(builder).await
    }

    async fn update_and_autoposition_task(&self, task: Task) -> Result<Task, StorageError> {
        let id = task.id.to_string();
        let builder = self
            .request(Method::POST, &["tasks", &id, "move"])?
            .json(&task);
        self.send_json(builder).await
    }

    async fn delete_task(&self, id: TaskId) -> Result<(), StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::DELETE, &["tasks", &id])?;
        self.send_empty(builder).await
    }

    async fn create_note(&self, task_id: TaskId, contents: String) -> Result<Note, StorageError> {
        let task_id = task_id.to_string();
        let body = CreateNoteBody { contents };
        let builder = self
            .request(Method::POST, &["tasks", &task_id, "notes"])?
            .json(&body);
        self.send_json(builder).await
    }

    async fn get_note_by_id(&self, id: NoteId) -> Result<Option<Note>, StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::GET, &["notes", &id])?;
        self.send_json(builder).await
    }

    async fn get_all_notes_by_task_id(&self, task_id: TaskId) -> Result<Vec<Note>, StorageError> {
        let task_id = task_id.to_string();
        let builder = self.request(Method::GET, &["tasks", &task_id, "notes"])?;
        self.send_json(builder).await
    }

    async fn get_all_notes_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Note>, StorageError> {
        let project_id = project_id.to_string();
        let builder = self.request(Method::GET, &["projects", &project_id, "notes"])?;
        self.send_json(builder).await
    }

    async fn update_note(&self, note: Note) -> Result<Note, StorageError> {
        let id = note.id.to_string();
        let builder = self.request(Method::PUT, &["notes", &id])?.json(&note);
        self.send_json(builder).await
    }

    async fn delete_note(&self, id: NoteId) -> Result<(), StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::DELETE, &["notes", &id])?;
        self.send_empty(builder).await
    }

    async fn create_status(
        &self,
        project_id: ProjectId,
        name: String,
        position: i32,
        color: Option<Color>,
        style: StatusStyle,
    ) -> Result<Status, StorageError> {
        let project_id = project_id.to_string();
        let body = CreateStatusBody {
            name,
            position,
            color,
            style,
        };
        let builder = self
            .request(Method::POST, &["projects", &project_id, "statuses"])?
            .json(&body);
        self.send_json(builder).await
    }

    async fn get_status_by_id(&self, id: StatusId) -> Result<Option<Status>, StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::GET, &["statuses", &id])?;
        self.send_json(builder).await
    }

    async fn get_status_by_project_id_and_status_name(
        &self,
        project_id: ProjectId,
        status_name: String,
    ) -> Result<Option<Status>, StorageError> {
        let project_id = project_id.to_string();
        let builder = self.request(
            Method::GET,
            &["projects", &project_id, "statuses", "by-name", &status_name],
        )?;
        self.send_json(builder).await
    }

    async fn get_all_statuses_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Status>, StorageError> {
        let project_id = project_id.to_string();
        let builder = self.request(Method::GET, &["projects", &project_id, "statuses"])?;
        self.send_json(builder).await
    }

    async fn update_status(&self, status: Status) -> Result<Status, StorageError> {
        let id = status.id.to_string();
        let builder = self.request(Method::PUT, &["statuses", &id])?.json(&status);
        self.send_json(builder).await
    }

    async fn reorder_status(
        &self,
        _project_id: ProjectId,
        status_id: StatusId,
        new_position: i32,
    ) -> Result<(), StorageError> {
        // the server derives the project from the status, so project_id is unused
        let status_id = status_id.to_string();
        let body = MoveStatusBody {
            position: new_position,
        };
        let builder = self
            .request(Method::POST, &["statuses", &status_id, "move"])?
            .json(&body);
        self.send_empty(builder).await
    }

    async fn delete_status(&self, id: StatusId) -> Result<(), StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::DELETE, &["statuses", &id])?;
        self.send_empty(builder).await
    }

    async fn create_project(
        &self,
        name: String,
        entry_status_id: Option<StatusId>,
        task_sorting_mode: TaskSortingMode,
        show_priority: bool,
    ) -> Result<Project, StorageError> {
        let body = CreateProjectBody {
            name,
            entry_status_id,
            task_sorting_mode,
            show_priority,
        };
        let builder = self.request(Method::POST, &["projects"])?.json(&body);
        self.send_json(builder).await
    }

    async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        let id = id.to_string();
        let builder = self.request(Method::GET, &["projects", &id])?;
        self.send_json(builder).await
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, StorageError> {
        let builder = self.request(Method::GET, &["projects", "by-name", name])?;
        self.send_json(builder).await
    }

    async fn get_all_projects(&self) -> Result<Vec<Project>, StorageError> {
        let builder = self.request(Method::GET, &["projects"])?;
        self.send_json(builder).await
    }

    async fn update_project(&self, project: Project) -> Result<Project, StorageError> {
        let id = project.id.to_string();
        let builder = self
            .request(Method::PUT, &["projects", &id])?
            .json(&project);
        self.send_json(builder).await
    }

    async fn delete_project(&self, name: String) -> Result<(), StorageError> {
        let builder = self.request(Method::DELETE, &["projects", "by-name", &name])?;
        self.send_empty(builder).await
    }

    async fn import(&self, plan: ImportPlan) -> Result<ImportReport, StorageError> {
        let builder = self.request(Method::POST, &["import"])?.json(&plan);
        self.send_json(builder).await
    }
}

#[cfg(test)]
mod tests {
    use super::error_from_response;
    use reqwest::StatusCode;
    use scry_core::error::StorageError;

    #[test]
    fn maps_error_kinds_from_the_flat_envelope() {
        let envelope = |kind: &str| format!(r#"{{"code":0,"kind":"{kind}","message":"boom"}}"#);

        assert!(matches!(
            error_from_response(StatusCode::NOT_FOUND, &envelope("not_found")),
            StorageError::NotFound(message) if message == "boom"
        ));
        assert!(matches!(
            error_from_response(StatusCode::CONFLICT, &envelope("conflict")),
            StorageError::Conflict(message) if message == "boom"
        ));
        assert!(matches!(
            error_from_response(StatusCode::BAD_REQUEST, &envelope("invalid")),
            StorageError::Invalid(message) if message == "boom"
        ));
        assert!(matches!(
            error_from_response(StatusCode::INTERNAL_SERVER_ERROR, &envelope("internal")),
            StorageError::Database(message) if message == "boom"
        ));
    }

    #[test]
    fn falls_back_to_the_raw_body_when_it_does_not_parse() {
        let error = error_from_response(StatusCode::BAD_GATEWAY, "not json");

        assert!(matches!(error, StorageError::Database(message) if message.contains("not json")));
    }
}
