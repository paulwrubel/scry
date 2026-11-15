use crate::error::ServiceError;
use crate::service::placement::section_placement_position;
use scry_core::ids::{ProjectId, SectionId};
use scry_core::models::{Color, Section};
use scry_core::position::n_keys_between;
use scry_core::sync::operations::{ColorChange as OperationColorChange, OperationKind};
use uuid::Uuid;

use super::super::{
    ApplyMode, ColorChange, ProjectService, SectionPlacement, UpdateSection, new_operation,
    position_error,
};

impl ProjectService {
    pub async fn create_section_in_project(
        &self,
        project_id: ProjectId,
        name: String,
        color: Option<Color>,
    ) -> Result<Section, ServiceError> {
        let project = self.require_project(project_id).await?;
        let (section, operation) = self.build_section(&project, name, color).await?;
        self.apply_operations(&[operation], ApplyMode::Atomic)
            .await?;
        Ok(section)
    }

    /// Look up a section by its exact name within a project.
    pub async fn get_section_by_name(
        &self,
        project_id: ProjectId,
        name: &str,
    ) -> Result<Section, ServiceError> {
        match self
            .store
            .get_section_by_name(project_id, name, None)
            .await?
        {
            Some(section) => Ok(section),
            None => {
                let project = self.require_project(project_id).await?;
                Err(ServiceError::SectionNotFound {
                    section: name.to_string(),
                    project: project.name,
                })
            }
        }
    }

    pub async fn get_section(&self, id: SectionId) -> Result<Option<Section>, ServiceError> {
        Ok(self.store.get_section_by_id(id, None).await?)
    }

    pub async fn list_sections_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Section>, ServiceError> {
        Ok(self
            .store
            .get_all_sections_by_project_id(project_id, None)
            .await?)
    }

    /// Update a section's name and color, leaving untouched fields alone.
    pub async fn update_section(
        &self,
        section_id: SectionId,
        update: UpdateSection,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let name = match update.name {
            Some(name) => {
                let project = self.require_project(section.project_id).await?;
                self.validated_section_name(&project, Some(section.id), name)
                    .await?
            }
            None => section.name.clone(),
        };
        let operation_color = match update.color {
            ColorChange::Clear => OperationColorChange::Clear,
            ColorChange::Set(color) => OperationColorChange::Set(color),
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateSection {
                id: section_id,
                name: Some(name),
                color: Some(operation_color),
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_section(section_id).await
    }

    /// Move a section to the top, bottom, or next to a sibling.
    pub async fn move_section(
        &self,
        section_id: SectionId,
        placement: SectionPlacement,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let project = self.require_project(section.project_id).await?;
        let anchor_id = match placement {
            SectionPlacement::Before(anchor_id) | SectionPlacement::After(anchor_id) => {
                Some(anchor_id)
            }
            _ => None,
        };
        if let Some(anchor_id) = anchor_id {
            self.require_section_in_project(&project, anchor_id).await?;
            if anchor_id == section.id {
                return Err(ServiceError::SectionMoveAnchorItself {
                    section: section.name.clone(),
                });
            }
        }
        let sections = self
            .store
            .get_all_sections_by_project_id(project.id, None)
            .await?;
        let Some(new_position) = section_placement_position(&sections, &section, placement)? else {
            return Ok(section);
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::MoveSection {
                id: section.id,
                position: new_position,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_section(section_id).await
    }

    pub async fn delete_section_by_id(
        &self,
        section_id: SectionId,
    ) -> Result<Section, ServiceError> {
        let section = self.require_section(section_id).await?;
        let automations = self
            .store
            .get_all_automations_by_project_id(section.project_id, None)
            .await?;
        if automations
            .iter()
            .any(|automation| automation.references_section(section.id))
        {
            return Err(ServiceError::SectionInUseByAutomation {
                section: section.name.clone(),
            });
        }
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteSection { id: section_id },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(section)
    }

    pub async fn merge_sections_by_id(
        &self,
        source_id: SectionId,
        target_id: SectionId,
    ) -> Result<(Section, Section), ServiceError> {
        let source = self.require_section(source_id).await?;
        let project = self.require_project(source.project_id).await?;
        let target = self.require_section_in_project(&project, target_id).await?;
        if source.id == target.id {
            return Err(ServiceError::SectionMergeIntoItself {
                section: source.name.clone(),
            });
        }

        let automations = self
            .store
            .get_all_automations_by_project_id(project.id, None)
            .await?;
        if let Some(automation) = automations.iter().find(|automation| {
            automation.references_section(source.id) || automation.references_section(target.id)
        }) {
            let section = if automation.references_section(source.id) {
                source.name.clone()
            } else {
                target.name.clone()
            };
            return Err(ServiceError::SectionInUseByAutomation { section });
        }

        let source_tasks = self.tasks_in_section(project.id, Some(source.id)).await?;
        let last_target = self.last_task_position(project.id, Some(target.id)).await?;
        let positions = n_keys_between(last_target.as_ref(), None, source_tasks.len())
            .map_err(position_error)?;
        let mut ops = Vec::with_capacity(source_tasks.len() + 1);
        for (task, position) in source_tasks.into_iter().zip(positions) {
            ops.push(new_operation(
                Uuid::new_v4(),
                OperationKind::MoveTask {
                    id: task.id,
                    section_id: Some(target.id),
                    position,
                    updated_at: None,
                },
            ));
        }
        ops.push(new_operation(
            Uuid::new_v4(),
            OperationKind::DeleteSection { id: source.id },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok((source, target))
    }
}
