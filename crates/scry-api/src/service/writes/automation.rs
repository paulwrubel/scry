use crate::error::ServiceError;
use chrono::Utc;
use scry_core::ids::{AutomationId, ProjectId};
use scry_core::models::{Automation, AutomationAction, AutomationTrigger};
use scry_core::sync::operations::OperationKind;
use uuid::Uuid;

use super::super::{ApplyMode, ProjectService, new_operation};

impl ProjectService {
    /// Create an automation in a project.
    pub async fn create_automation_in_project(
        &self,
        project_id: ProjectId,
        trigger: AutomationTrigger,
        actions: Vec<AutomationAction>,
    ) -> Result<Automation, ServiceError> {
        let project = self.require_project(project_id).await?;
        let created_at = Utc::now();
        let automation = Automation {
            id: Uuid::new_v4(),
            project_id: project.id,
            trigger,
            actions: actions.clone(),
            created_at,
            updated_at: created_at,
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateAutomation {
                id: automation.id,
                project_id: automation.project_id,
                trigger: automation.trigger.clone(),
                actions,
                created_at: automation.created_at,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(automation)
    }

    /// Replace an automation's action list. The trigger is immutable.
    pub async fn update_automation(
        &self,
        id: AutomationId,
        actions: Vec<AutomationAction>,
    ) -> Result<Automation, ServiceError> {
        self.store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateAutomation {
                id,
                actions,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })
    }

    /// Delete an automation, returning the deleted rule.
    pub async fn delete_automation(&self, id: AutomationId) -> Result<Automation, ServiceError> {
        let existing = self
            .store
            .get_automation_by_id(id, None)
            .await?
            .ok_or(ServiceError::AutomationNotFound { id })?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteAutomation { id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(existing)
    }

    pub async fn get_automation(
        &self,
        id: AutomationId,
    ) -> Result<Option<Automation>, ServiceError> {
        Ok(self.store.get_automation_by_id(id, None).await?)
    }

    pub async fn list_automations(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Automation>, ServiceError> {
        Ok(self
            .store
            .get_all_automations_by_project_id(project_id, None)
            .await?)
    }
}
