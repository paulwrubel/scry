//! Handler for fetching a single project.

use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use scry_core::error::StorageError;
use scry_core::ids::TaskId;
use scry_core::models::{Note, Project, ProjectId, Section};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::server::ScryState;

use super::TaskDetails;

/// Query options for the project detail endpoint.
#[derive(Deserialize, Default)]
pub struct GetProjectQuery {
    /// Include the project's sections and tasks.
    #[serde(default)]
    pub detailed: bool,
    /// Include each task's display handle. Only used with `detailed`.
    #[serde(default)]
    pub include_handles: bool,
}

/// A project plus its related data, with the project fields flattened.
#[derive(Serialize)]
pub struct ProjectDetailResponse {
    #[serde(flatten)]
    pub project: Project,
    pub sections: Vec<Section>,
    pub tasks: Vec<TaskDetails>,
}

/// Fetch a project by id, optionally with its sections and tasks.
pub async fn get_project(
    State(state): State<ScryState>,
    Path(id): Path<ProjectId>,
    Query(query): Query<GetProjectQuery>,
) -> Response {
    if query.detailed {
        let snapshot = match state.service.project_snapshot(id).await {
            Ok(snapshot) => snapshot,
            Err(error) => return AppError::from(error).into(),
        };

        let mut notes_by_task: HashMap<TaskId, Vec<Note>> = HashMap::new();
        for note in snapshot.notes {
            notes_by_task.entry(note.task_id).or_default().push(note);
        }

        let handles = if query.include_handles {
            match state.service.list_task_handles().await {
                Ok(handles) => {
                    let task_ids: HashSet<TaskId> =
                        snapshot.tasks.iter().map(|task| task.id).collect();
                    handles
                        .into_iter()
                        .filter(|(id, _)| task_ids.contains(id))
                        .collect()
                }
                Err(error) => return AppError::from(error).into(),
            }
        } else {
            HashMap::new()
        };

        let tasks = snapshot
            .tasks
            .into_iter()
            .map(|task| {
                let notes = notes_by_task.remove(&task.id).unwrap_or_default();
                let handle = handles.get(&task.id).cloned();
                TaskDetails {
                    task,
                    notes: Some(notes),
                    handle,
                }
            })
            .collect();

        return Json(ProjectDetailResponse {
            project: snapshot.project,
            sections: snapshot.sections,
            tasks,
        })
        .into_response();
    }

    match state.service.get_project(id).await {
        Ok(Some(project)) => Json(project).into_response(),
        Ok(None) => {
            AppError::Storage(StorageError::NotFound(format!("project {id} not found"))).into()
        }
        Err(error) => AppError::from(error).into(),
    }
}
