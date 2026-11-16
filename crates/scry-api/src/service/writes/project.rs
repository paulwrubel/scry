use crate::error::ServiceError;
use jiff::Timestamp;
use scry_core::error::StorageError;
use scry_core::ids::ProjectId;
use scry_core::models::{Project, ProjectSnapshot};
use scry_core::sync::operations::OperationKind;
use uuid::Uuid;

use super::super::{ApplyMode, ProjectService, UpdateProject, new_operation};

impl ProjectService {
    /// Create a project.
    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Project, ServiceError> {
        let created_at = Timestamp::now();
        let project = Project {
            id: Uuid::new_v4(),
            name,
            description: description.filter(|description| !description.is_empty()),
            created_at,
            updated_at: created_at,
        };

        if self.get_project_by_name(&project.name).await?.is_some() {
            return Err(StorageError::Conflict(format!(
                "project '{}' already exists",
                project.name
            ))
            .into());
        }

        let ops = vec![new_operation(
            Uuid::new_v4(),
            OperationKind::CreateProject {
                id: project.id,
                name: project.name.clone(),
                description: project.description.clone(),
                created_at: project.created_at,
                updated_at: None,
            },
        )];

        self.apply_operations(&ops, ApplyMode::Atomic).await?;
        Ok(project)
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.store.get_all_projects(None).await?)
    }

    pub async fn get_project(&self, id: ProjectId) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_id(id, None).await?)
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, ServiceError> {
        Ok(self.store.get_project_by_name(name, None).await?)
    }

    pub async fn project_snapshot(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectSnapshot, ServiceError> {
        Ok(self.store.get_project_snapshot(project_id, None).await?)
    }

    pub async fn update_project(
        &self,
        project: &Project,
        update: UpdateProject,
    ) -> Result<Project, ServiceError> {
        let description = match update.description.clone() {
            Some(value) => Some(value).filter(|value| !value.is_empty()),
            None => project.description.clone(),
        };
        let updated = Project {
            name: update.name.clone().unwrap_or_else(|| project.name.clone()),
            description,
            ..project.clone()
        };

        let changed = update.name.is_some() || update.description.is_some();
        if changed {
            let description = update
                .description
                .map(|value| Some(value).filter(|value| !value.is_empty()));
            let op = new_operation(
                Uuid::new_v4(),
                OperationKind::UpdateProject {
                    id: project.id,
                    name: update.name,
                    description,
                    updated_at: None,
                },
            );
            self.apply_operations(&[op], ApplyMode::Atomic).await?;
            return self.require_project(project.id).await;
        }
        Ok(updated)
    }

    /// Delete a project by id, returning the deleted project.
    pub async fn delete_project_by_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Project, ServiceError> {
        let project = self.require_project(project_id).await?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteProject { id: project.id },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(project)
    }
}
