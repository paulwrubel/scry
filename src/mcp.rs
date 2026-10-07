//! In-process Model Context Protocol (MCP) server exposing scry operations as tools.

use crate::error::AppError;
use crate::store::TaskStore;
use crate::store::sqlite::SqliteStore;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use serde::Serialize;

mod http;
mod tools;

pub use http::serve_http;

/// Build a caller-visible tool-level error result from an application error.
///
/// The payload mirrors the CLI's `--json` error document so the same `kind`
/// vocabulary appears on both surfaces.
pub(crate) fn error_result(error: impl Into<AppError>) -> CallToolResult {
    let error = error.into();
    let document = serde_json::json!({
        "error": {
            "kind": error.kind(),
            "message": error.to_string(),
        }
    });
    CallToolResult::error(vec![ContentBlock::text(document.to_string())])
}

/// Build a success result whose content is the compact JSON of `value`.
pub(crate) fn success_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value).map_err(|error| {
        McpError::internal_error(format!("failed to serialize tool result: {error}"), None)
    })?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Convert a store or service result into a tool result: `Ok` becomes JSON
/// content, `Err` becomes a caller-visible tool-level error.
pub(crate) fn tool_result<T, E>(result: Result<T, E>) -> Result<CallToolResult, McpError>
where
    T: Serialize,
    E: Into<AppError>,
{
    match result {
        Ok(value) => success_result(&value),
        Err(error) => Ok(error_result(error)),
    }
}

/// The scry MCP server. Holds the store that tool handlers operate on.
#[derive(Clone)]
pub struct ScryServer {
    store: SqliteStore,
    tool_router: ToolRouter<ScryServer>,
}

#[tool_router]
impl ScryServer {
    pub fn new(store: SqliteStore) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Report scry server information, including the active project")]
    async fn scry_info(&self) -> Result<CallToolResult, McpError> {
        let project = match self.store.get_active_project().await {
            Ok(project) => project,
            Err(error) => return Ok(error_result(error)),
        };
        success_result(&serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "active_project": project.name,
        }))
    }

    #[tool(description = "List all projects")]
    async fn project_list(&self) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::list_projects(&self.store).await)
    }

    #[tool(description = "Show the active project")]
    async fn project_current(&self) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::active_project(&self.store).await)
    }

    #[tool(description = "List the statuses of a project")]
    async fn status_list(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusListParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::list_statuses(&self.store, &params).await)
    }

    #[tool(
        description = "List tasks in a project, optionally filtered by status and by title or tag search"
    )]
    async fn task_list(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskListParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::list_tasks(&self.store, &params).await)
    }

    #[tool(description = "Show full details for a task, including its notes")]
    async fn task_show(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskShowParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::show_task(&self.store, &params).await)
    }

    #[tool(description = "Add a task to a project")]
    async fn task_add(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::add_task(&self.store, &params).await)
    }

    #[tool(description = "Update a task's title, description, priority, tags, or status")]
    async fn task_update(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::update_task(&self.store, &params).await)
    }

    #[tool(description = "Move a task to a different status")]
    async fn task_move(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::move_task(&self.store, &params).await)
    }

    #[tool(description = "Duplicate a task into the same status")]
    async fn task_duplicate(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskDuplicateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::duplicate_task(&self.store, &params).await)
    }

    #[tool(
        description = "Permanently delete a task",
        annotations(destructive_hint = true)
    )]
    async fn task_delete(
        &self,
        Parameters(params): Parameters<tools::tasks::TaskDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::tasks::delete_task(&self.store, &params).await)
    }

    #[tool(description = "Add a note to a task")]
    async fn note_add(
        &self,
        Parameters(params): Parameters<tools::notes::NoteAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::notes::add_note(&self.store, &params).await)
    }

    #[tool(description = "Create a project, optionally from a template (todolist or kanban)")]
    async fn project_create(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::create_project(&self.store, &params).await)
    }

    #[tool(description = "Rename a project")]
    async fn project_rename(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectRenameParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::rename_project(&self.store, &params).await)
    }

    #[tool(description = "Set the active project")]
    async fn project_use(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectUseParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::use_project(&self.store, &params).await)
    }

    #[tool(
        description = "Permanently delete a project and all of its tasks",
        annotations(destructive_hint = true)
    )]
    async fn project_delete(
        &self,
        Parameters(params): Parameters<tools::projects::ProjectDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::projects::delete_project(&self.store, &params).await)
    }

    #[tool(
        description = "Set or clear the project's entry status (the status new tasks default to)"
    )]
    async fn project_set_entry_status(
        &self,
        Parameters(params): Parameters<tools::project_settings::ProjectSetEntryStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::project_settings::set_entry_status(&self.store, &params).await)
    }

    #[tool(description = "Set the project's task sorting mode")]
    async fn project_set_sort_mode(
        &self,
        Parameters(params): Parameters<tools::project_settings::ProjectSetSortModeParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::project_settings::set_sort_mode(&self.store, &params).await)
    }

    #[tool(description = "Show or hide task priority in the project's listings")]
    async fn project_set_show_priority(
        &self,
        Parameters(params): Parameters<tools::project_settings::ProjectSetShowPriorityParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::project_settings::set_show_priority(&self.store, &params).await)
    }

    #[tool(description = "Add a status to a project")]
    async fn status_add(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusAddParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::add_status(&self.store, &params).await)
    }

    #[tool(description = "Rename a status")]
    async fn status_rename(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusRenameParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::rename_status(&self.store, &params).await)
    }

    #[tool(description = "Move a status up or down within a project's ordering")]
    async fn status_move(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::move_status(&self.store, &params).await)
    }

    #[tool(description = "Set or clear a status's color")]
    async fn status_set_color(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusSetColorParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::set_status_color(&self.store, &params).await)
    }

    #[tool(description = "Set a status's style")]
    async fn status_set_style(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusSetStyleParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::set_status_style(&self.store, &params).await)
    }

    #[tool(
        description = "Remove an empty status from a project",
        annotations(destructive_hint = true)
    )]
    async fn status_remove(
        &self,
        Parameters(params): Parameters<tools::statuses::StatusRemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(tools::statuses::remove_status(&self.store, &params).await)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ScryServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("scry", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "scry is a task manager. Use these tools to manage tasks, projects, statuses, and notes.",
            )
    }
}

/// Serve the MCP protocol over stdio until the client disconnects.
pub async fn serve_stdio(store: SqliteStore) -> Result<(), AppError> {
    let server = ScryServer::new(store);
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|error| AppError::Internal(format!("failed to start MCP server: {error}")))?;
    service
        .waiting()
        .await
        .map_err(|error| AppError::Internal(format!("MCP server error: {error}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{ProjectService, TaskInput};
    use assert_fs::TempDir;

    async fn test_server() -> (TempDir, ScryServer) {
        let dir = TempDir::new().expect("temp dir");
        let url = format!("sqlite://{}", dir.path().join("scry.db").display());
        let store = SqliteStore::new(&url).await.expect("store");
        (dir, ScryServer::new(store))
    }

    fn text_of(result: &CallToolResult) -> String {
        let value = serde_json::to_value(result).expect("serialize result");
        value["content"][0]["text"]
            .as_str()
            .expect("text content block")
            .to_string()
    }

    async fn add_task(server: &ScryServer, title: &str) -> i64 {
        let project = server
            .store
            .get_active_project()
            .await
            .expect("active project");
        let service = ProjectService::new(&server.store);
        let change = service
            .create_task(
                &project,
                TaskInput {
                    title: Some(title.to_string()),
                    ..Default::default()
                },
            )
            .await
            .expect("create task");
        change.task.id
    }

    #[tokio::test]
    async fn scry_info_reports_version_and_active_project() {
        let (_dir, server) = test_server().await;
        let result = server.scry_info().await.expect("scry_info");
        let json = serde_json::to_string(&result).expect("serialize result");

        assert!(
            json.contains(env!("CARGO_PKG_VERSION")),
            "version missing from result: {json}"
        );
        assert!(
            json.contains("default"),
            "active project missing from result: {json}"
        );
    }

    #[test]
    fn error_result_carries_the_stable_kind_and_message() {
        let error = AppError::Service(crate::error::ServiceError::TaskNotFound {
            task_id: 7,
            project: "default".to_string(),
        });

        let result = error_result(error);

        assert_eq!(result.is_error, Some(true), "expected a tool-level error");
        let value = serde_json::to_value(&result).expect("serialize result");
        let text = value["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert!(
            text.contains("\"kind\":\"not_found\""),
            "kind missing: {text}"
        );
        assert!(text.contains("Task 7 not found"), "message missing: {text}");
    }

    #[tokio::test]
    async fn project_list_returns_projects() {
        let (_dir, server) = test_server().await;
        let result = server.project_list().await.expect("project_list");
        assert!(
            text_of(&result).contains("\"name\":\"default\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn project_current_returns_the_active_project() {
        let (_dir, server) = test_server().await;
        let result = server.project_current().await.expect("project_current");
        assert!(
            text_of(&result).contains("\"name\":\"default\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn status_list_defaults_to_the_active_project() {
        let (_dir, server) = test_server().await;
        let result = server
            .status_list(Parameters(tools::statuses::StatusListParams {
                project: None,
            }))
            .await
            .expect("status_list");
        let text = text_of(&result);
        assert!(text.contains("\"name\":\"todo\""), "{text}");
        assert!(text.contains("\"name\":\"done\""), "{text}");
    }

    #[tokio::test]
    async fn status_list_reports_unknown_projects_as_tool_errors() {
        let (_dir, server) = test_server().await;
        let result = server
            .status_list(Parameters(tools::statuses::StatusListParams {
                project: Some("nope".to_string()),
            }))
            .await
            .expect("status_list");
        assert_eq!(result.is_error, Some(true));
        let text = text_of(&result);
        assert!(text.contains("\"kind\":\"not_found\""), "{text}");
        assert!(text.contains("project 'nope' not found"), "{text}");
    }

    #[tokio::test]
    async fn task_list_returns_tasks() {
        let (_dir, server) = test_server().await;
        add_task(&server, "Alpha").await;

        let result = server
            .task_list(Parameters(tools::tasks::TaskListParams {
                status: None,
                search: None,
                project: None,
            }))
            .await
            .expect("task_list");
        assert!(text_of(&result).contains("Alpha"), "{}", text_of(&result));
    }

    #[tokio::test]
    async fn task_list_applies_the_search_filter() {
        let (_dir, server) = test_server().await;
        add_task(&server, "Alpha").await;
        add_task(&server, "Beta").await;

        let result = server
            .task_list(Parameters(tools::tasks::TaskListParams {
                status: None,
                search: Some("alp".to_string()),
                project: None,
            }))
            .await
            .expect("task_list");
        let text = text_of(&result);
        assert!(text.contains("Alpha"), "{text}");
        assert!(!text.contains("Beta"), "{text}");
    }

    #[tokio::test]
    async fn task_show_includes_notes() {
        let (_dir, server) = test_server().await;
        let id = add_task(&server, "Alpha").await;
        let project = server
            .store
            .get_active_project()
            .await
            .expect("active project");
        ProjectService::new(&server.store)
            .add_task_note(&project, id, "a note".to_string())
            .await
            .expect("add note");

        let result = server
            .task_show(Parameters(tools::tasks::TaskShowParams {
                id,
                project: None,
            }))
            .await
            .expect("task_show");
        let text = text_of(&result);
        assert!(text.contains("Alpha"), "{text}");
        assert!(text.contains("a note"), "{text}");
    }

    #[tokio::test]
    async fn task_show_reports_unknown_ids_as_tool_errors() {
        let (_dir, server) = test_server().await;
        let result = server
            .task_show(Parameters(tools::tasks::TaskShowParams {
                id: 999,
                project: None,
            }))
            .await
            .expect("task_show");
        assert_eq!(result.is_error, Some(true));
        assert!(text_of(&result).contains("\"kind\":\"not_found\""));
    }

    #[tokio::test]
    async fn task_add_creates_a_task() {
        let (_dir, server) = test_server().await;
        let result = server
            .task_add(Parameters(tools::tasks::TaskAddParams {
                title: "Alpha".to_string(),
                description: None,
                priority: Some(crate::models::Priority::High),
                tags: None,
                status: None,
                project: None,
            }))
            .await
            .expect("task_add");
        let text = text_of(&result);
        assert!(text.contains("Alpha"), "{text}");
        assert!(text.contains("\"priority\":\"high\""), "{text}");
    }

    #[tokio::test]
    async fn task_update_changes_fields() {
        let (_dir, server) = test_server().await;
        let id = add_task(&server, "Alpha").await;

        let result = server
            .task_update(Parameters(tools::tasks::TaskUpdateParams {
                id,
                title: Some("Renamed".to_string()),
                description: None,
                priority: None,
                tags: None,
                status: None,
                project: None,
            }))
            .await
            .expect("task_update");
        assert!(text_of(&result).contains("Renamed"), "{}", text_of(&result));
    }

    #[tokio::test]
    async fn task_update_requires_at_least_one_field() {
        let (_dir, server) = test_server().await;
        let id = add_task(&server, "Alpha").await;

        let result = server
            .task_update(Parameters(tools::tasks::TaskUpdateParams {
                id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                status: None,
                project: None,
            }))
            .await
            .expect("task_update");
        assert_eq!(result.is_error, Some(true));
        assert!(
            text_of(&result).contains("\"kind\":\"invalid\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn task_move_duplicate_and_delete_round_trip() {
        let (_dir, server) = test_server().await;
        let id = add_task(&server, "Alpha").await;

        let moved = server
            .task_move(Parameters(tools::tasks::TaskMoveParams {
                id,
                status: "done".to_string(),
                project: None,
            }))
            .await
            .expect("task_move");
        assert!(
            text_of(&moved).contains("\"status_id\":2"),
            "{}",
            text_of(&moved)
        );

        let duplicated = server
            .task_duplicate(Parameters(tools::tasks::TaskDuplicateParams {
                id,
                project: None,
            }))
            .await
            .expect("task_duplicate");
        assert!(
            text_of(&duplicated).contains("Alpha"),
            "{}",
            text_of(&duplicated)
        );

        server
            .task_delete(Parameters(tools::tasks::TaskDeleteParams {
                id,
                project: None,
            }))
            .await
            .expect("task_delete");

        let missing = server
            .task_show(Parameters(tools::tasks::TaskShowParams {
                id,
                project: None,
            }))
            .await
            .expect("task_show");
        assert_eq!(missing.is_error, Some(true));
    }

    #[tokio::test]
    async fn note_add_attaches_a_note_to_a_task() {
        let (_dir, server) = test_server().await;
        let id = add_task(&server, "Alpha").await;

        let result = server
            .note_add(Parameters(tools::notes::NoteAddParams {
                task_id: id,
                contents: "a note".to_string(),
                project: None,
            }))
            .await
            .expect("note_add");
        assert!(text_of(&result).contains("a note"), "{}", text_of(&result));

        let shown = server
            .task_show(Parameters(tools::tasks::TaskShowParams {
                id,
                project: None,
            }))
            .await
            .expect("task_show");
        assert!(text_of(&shown).contains("a note"), "{}", text_of(&shown));
    }

    #[tokio::test]
    async fn project_create_seeds_template_statuses() {
        let (_dir, server) = test_server().await;

        let created = server
            .project_create(Parameters(tools::projects::ProjectCreateParams {
                name: "todolist".to_string(),
                template: Some("todolist".to_string()),
            }))
            .await
            .expect("project_create");
        assert!(
            text_of(&created).contains("\"name\":\"todolist\""),
            "{}",
            text_of(&created)
        );

        let statuses = server
            .status_list(Parameters(tools::statuses::StatusListParams {
                project: Some("todolist".to_string()),
            }))
            .await
            .expect("status_list");
        let text = text_of(&statuses);
        assert!(text.contains("\"name\":\"todo\""), "{text}");
        assert!(text.contains("\"name\":\"done\""), "{text}");
    }

    #[tokio::test]
    async fn project_create_rejects_unknown_templates() {
        let (_dir, server) = test_server().await;

        let result = server
            .project_create(Parameters(tools::projects::ProjectCreateParams {
                name: "mystery".to_string(),
                template: Some("nope".to_string()),
            }))
            .await
            .expect("project_create");
        assert_eq!(result.is_error, Some(true));
        assert!(
            text_of(&result).contains("\"kind\":\"invalid\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn project_rename_changes_the_name() {
        let (_dir, server) = test_server().await;
        server
            .project_create(Parameters(tools::projects::ProjectCreateParams {
                name: "alpha".to_string(),
                template: None,
            }))
            .await
            .expect("project_create");

        let renamed = server
            .project_rename(Parameters(tools::projects::ProjectRenameParams {
                old_name: "alpha".to_string(),
                new_name: "beta".to_string(),
            }))
            .await
            .expect("project_rename");
        assert!(
            text_of(&renamed).contains("\"name\":\"beta\""),
            "{}",
            text_of(&renamed)
        );
    }

    #[tokio::test]
    async fn project_use_switches_the_active_project() {
        let (_dir, server) = test_server().await;
        server
            .project_create(Parameters(tools::projects::ProjectCreateParams {
                name: "alpha".to_string(),
                template: None,
            }))
            .await
            .expect("project_create");

        server
            .project_use(Parameters(tools::projects::ProjectUseParams {
                name: "alpha".to_string(),
            }))
            .await
            .expect("project_use");

        let current = server.project_current().await.expect("project_current");
        assert!(
            text_of(&current).contains("\"name\":\"alpha\""),
            "{}",
            text_of(&current)
        );
    }

    #[tokio::test]
    async fn project_delete_removes_the_project() {
        let (_dir, server) = test_server().await;
        server
            .project_create(Parameters(tools::projects::ProjectCreateParams {
                name: "alpha".to_string(),
                template: None,
            }))
            .await
            .expect("project_create");

        let deleted = server
            .project_delete(Parameters(tools::projects::ProjectDeleteParams {
                name: "alpha".to_string(),
            }))
            .await
            .expect("project_delete");
        assert_eq!(deleted.is_error, Some(false));

        let listed = server.project_list().await.expect("project_list");
        let text = text_of(&listed);
        assert!(text.contains("default"), "{text}");
        assert!(!text.contains("\"name\":\"alpha\""), "{text}");
    }

    #[tokio::test]
    async fn project_settings_round_trip() {
        let (_dir, server) = test_server().await;

        let entry = server
            .project_set_entry_status(Parameters(
                tools::project_settings::ProjectSetEntryStatusParams {
                    status: Some("done".to_string()),
                    project: None,
                },
            ))
            .await
            .expect("project_set_entry_status");
        assert!(
            text_of(&entry).contains("\"entry_status_id\":2"),
            "{}",
            text_of(&entry)
        );

        let sorted = server
            .project_set_sort_mode(Parameters(
                tools::project_settings::ProjectSetSortModeParams {
                    mode: crate::models::TaskSortingMode::Priority,
                    project: None,
                },
            ))
            .await
            .expect("project_set_sort_mode");
        assert!(
            text_of(&sorted).contains("\"task_sorting_mode\":\"priority\""),
            "{}",
            text_of(&sorted)
        );

        let shown = server
            .project_set_show_priority(Parameters(
                tools::project_settings::ProjectSetShowPriorityParams {
                    enabled: true,
                    project: None,
                },
            ))
            .await
            .expect("project_set_show_priority");
        assert!(
            text_of(&shown).contains("\"show_priority\":true"),
            "{}",
            text_of(&shown)
        );
    }

    #[tokio::test]
    async fn project_set_entry_status_clears_when_omitted() {
        let (_dir, server) = test_server().await;

        let cleared = server
            .project_set_entry_status(Parameters(
                tools::project_settings::ProjectSetEntryStatusParams {
                    status: None,
                    project: None,
                },
            ))
            .await
            .expect("project_set_entry_status");
        assert!(
            text_of(&cleared).contains("\"entry_status_id\":null"),
            "{}",
            text_of(&cleared)
        );
    }

    #[tokio::test]
    async fn status_write_tools_round_trip() {
        let (_dir, server) = test_server().await;

        let added = server
            .status_add(Parameters(tools::statuses::StatusAddParams {
                name: "review".to_string(),
                project: None,
            }))
            .await
            .expect("status_add");
        assert!(
            text_of(&added).contains("\"name\":\"review\""),
            "{}",
            text_of(&added)
        );

        let renamed = server
            .status_rename(Parameters(tools::statuses::StatusRenameParams {
                old_name: "review".to_string(),
                new_name: "qa".to_string(),
                project: None,
            }))
            .await
            .expect("status_rename");
        assert!(
            text_of(&renamed).contains("\"name\":\"qa\""),
            "{}",
            text_of(&renamed)
        );

        let colored = server
            .status_set_color(Parameters(tools::statuses::StatusSetColorParams {
                name: "qa".to_string(),
                color: Some(crate::models::Color::Blue),
                project: None,
            }))
            .await
            .expect("status_set_color");
        assert!(
            text_of(&colored).contains("\"color\":\"blue\""),
            "{}",
            text_of(&colored)
        );

        let styled = server
            .status_set_style(Parameters(tools::statuses::StatusSetStyleParams {
                name: "qa".to_string(),
                style: crate::models::StatusStyle::Checked,
                project: None,
            }))
            .await
            .expect("status_set_style");
        assert!(
            text_of(&styled).contains("\"style\":\"checked\""),
            "{}",
            text_of(&styled)
        );

        let moved = server
            .status_move(Parameters(tools::statuses::StatusMoveParams {
                name: "qa".to_string(),
                direction: tools::statuses::StatusMoveDirection::Up,
                project: None,
            }))
            .await
            .expect("status_move");
        assert!(
            text_of(&moved).contains("\"position\":1"),
            "{}",
            text_of(&moved)
        );

        let removed = server
            .status_remove(Parameters(tools::statuses::StatusRemoveParams {
                name: "qa".to_string(),
                project: None,
            }))
            .await
            .expect("status_remove");
        assert_eq!(removed.is_error, Some(false));

        let listed = server
            .status_list(Parameters(tools::statuses::StatusListParams {
                project: None,
            }))
            .await
            .expect("status_list");
        assert!(
            !text_of(&listed).contains("\"name\":\"qa\""),
            "{}",
            text_of(&listed)
        );
    }

    #[tokio::test]
    async fn status_remove_rejects_a_status_with_tasks() {
        let (_dir, server) = test_server().await;
        add_task(&server, "Alpha").await;

        let result = server
            .status_remove(Parameters(tools::statuses::StatusRemoveParams {
                name: "todo".to_string(),
                project: None,
            }))
            .await
            .expect("status_remove");
        assert_eq!(result.is_error, Some(true));
        assert!(
            text_of(&result).contains("\"kind\":\"conflict\""),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn http_transport_answers_the_initialize_handshake() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode, header};
        use tower::ServiceExt;

        let (_dir, server) = test_server().await;
        let app = http::router(&server.store);

        let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"scry-test","version":"0"}}}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::ACCEPT, "application/json, text/event-stream")
                    .body(Body::from(initialize))
                    .expect("build request"),
            )
            .await
            .expect("send request");

        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body = String::from_utf8(bytes.to_vec()).expect("utf-8 body");

        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("serverInfo"), "{body}");
        assert!(body.contains("scry"), "{body}");
    }
}
