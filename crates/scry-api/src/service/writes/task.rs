use crate::error::ServiceError;
use crate::service::fields::update_changes_locked_external_field;
use crate::service::placement::task_placement_position;
use jiff::Timestamp;
use scry_core::error::StorageError;
use scry_core::ids::{ProjectId, SectionId, TaskId};
use scry_core::models::{Project, Task, TaskState};
use scry_core::position::key_between;
use scry_core::sync::operations::OperationKind;
use uuid::Uuid;

use super::super::{
    ApplyMode, NewTask, ProjectService, SectionInput, TaskChangeResult, TaskPatch, TaskPlacement,
    create_task_operation, new_operation, position_error,
};

impl ProjectService {
    pub async fn create_task_in_project(
        &self,
        project_id: ProjectId,
        input: NewTask,
    ) -> Result<TaskChangeResult, ServiceError> {
        let project = self.require_project(project_id).await?;
        let (section, section_op) = self.resolve_section(&project, input.section).await?;
        let section_id = section.as_ref().map(|section| section.id);
        let position = self.append_position(project.id, section_id).await?;

        let created_at = Timestamp::now();
        let task = Task {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: input.title,
            description: input
                .description
                .filter(|description| !description.is_empty()),
            priority: input.priority.unwrap_or_default(),
            section_id,
            position: position.clone(),
            state: TaskState::Open,
            tags: input.tags.unwrap_or_default(),
            created_at,
            updated_at: created_at,
            external_identity: input.external_identity,
            external_metadata: input.external_metadata,
        };

        let mut ops = Vec::new();
        if let Some(section_op) = section_op {
            ops.push(section_op);
        }
        ops.push(new_operation(Uuid::new_v4(), create_task_operation(&task)));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn duplicate_task(
        &self,
        project: &Project,
        id: TaskId,
    ) -> Result<TaskChangeResult, ServiceError> {
        let source = self.require_task_in_project(project, id).await?;
        let tasks = self.tasks_in_section(project.id, source.section_id).await?;
        let next_position = tasks
            .iter()
            .position(|task| task.id == source.id)
            .and_then(|index| tasks.get(index + 1))
            .map(|task| &task.position);
        let position =
            key_between(Some(&source.position), next_position).map_err(position_error)?;

        let created_at = Timestamp::now();
        let task = Task {
            id: Uuid::new_v4(),
            project_id: source.project_id,
            title: source.title,
            description: source.description,
            priority: source.priority,
            section_id: source.section_id,
            position: position.clone(),
            state: source.state,
            tags: source.tags,
            created_at,
            updated_at: created_at,
            external_identity: source.external_identity.clone(),
            external_metadata: source.external_metadata.clone(),
        };

        let op = new_operation(Uuid::new_v4(), create_task_operation(&task));
        self.apply_operations(&[op], ApplyMode::Atomic).await?;

        Ok(TaskChangeResult { task })
    }

    pub async fn list_tasks_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self
            .store
            .get_all_tasks_by_project_id(project_id, None)
            .await?)
    }

    pub async fn list_tasks_by_section_id(
        &self,
        section_id: SectionId,
    ) -> Result<Vec<Task>, ServiceError> {
        Ok(self
            .store
            .get_all_tasks_by_section_id(section_id, None)
            .await?)
    }

    pub async fn update_task_by_id(
        &self,
        task_id: TaskId,
        patch: TaskPatch,
    ) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        let mut ops = Vec::new();
        let mut moved = false;

        if patch.section != SectionInput::Unchanged {
            let (section, section_op) = self.resolve_section(&project, patch.section).await?;
            if let Some(section_op) = section_op {
                ops.push(section_op);
            }
            let section_id = section.as_ref().map(|section| section.id);
            if section_id != task.section_id {
                task.position = self.append_position(project.id, section_id).await?;
                task.section_id = section_id;
                moved = true;
            }
        }

        if moved {
            ops.push(new_operation(
                Uuid::new_v4(),
                OperationKind::MoveTask {
                    id: task.id,
                    section_id: task.section_id,
                    position: task.position.clone(),
                    updated_at: None,
                },
            ));
        }

        if patch.title.is_some()
            || patch.description.is_some()
            || patch.priority.is_some()
            || patch.tags.is_some()
            || patch.state.is_some()
            || patch.external_identity.is_some()
            || patch.external_metadata.is_some()
        {
            let description = patch
                .description
                .map(|value| Some(value).filter(|value| !value.is_empty()));
            let kind = OperationKind::UpdateTask {
                id: task.id,
                title: patch.title,
                description,
                priority: patch.priority,
                tags: patch.tags,
                state: patch.state,
                external_identity: patch.external_identity,
                external_metadata: patch
                    .external_metadata
                    .map(|metadata| metadata.map(Box::new)),
                updated_at: None,
            };
            if update_changes_locked_external_field(&task, &kind) {
                return Err(ServiceError::Storage(StorageError::Conflict(format!(
                    "task {} is linked to an external identity; its title, description, and state are locked",
                    task.id
                ))));
            }
            ops.push(new_operation(Uuid::new_v4(), kind));
        }

        self.apply_operations(&ops, ApplyMode::Atomic).await?;
        self.require_task(task_id).await
    }

    pub async fn move_task_by_id(
        &self,
        task_id: TaskId,
        section: SectionInput,
        placement: Option<TaskPlacement>,
    ) -> Result<TaskChangeResult, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        let project = self.require_project(task.project_id).await?;
        let unchanged = section == SectionInput::Unchanged;
        let (section, section_op) = if unchanged {
            match task.section_id {
                Some(section_id) => (
                    Some(
                        self.require_section_in_project(&project, section_id)
                            .await?,
                    ),
                    None,
                ),
                None => (None, None),
            }
        } else {
            self.resolve_section(&project, section).await?
        };

        if unchanged && placement.is_none() {
            return Ok(TaskChangeResult { task });
        }

        let section_id = section.as_ref().map(|section| section.id);

        let position = match placement {
            Some(placement) => {
                if let TaskPlacement::Before(anchor_id) | TaskPlacement::After(anchor_id) =
                    placement
                {
                    if anchor_id == task.id {
                        return Err(ServiceError::TaskMoveAnchorItself { task: task.id });
                    }
                    let anchor = self.require_task_in_project(&project, anchor_id).await?;
                    if anchor.section_id != section_id {
                        let section_name = section
                            .as_ref()
                            .map(|section| section.name.clone())
                            .unwrap_or_else(|| "No section".to_string());
                        return Err(ServiceError::TaskAnchorNotInSection {
                            task: anchor_id,
                            section: section_name,
                        });
                    }
                }
                let tasks = self.tasks_in_section(project.id, section_id).await?;
                match task_placement_position(&tasks, &task, placement)? {
                    Some(position) => position,
                    None => return Ok(TaskChangeResult { task }),
                }
            }
            None => self.append_position(project.id, section_id).await?,
        };
        task.position = position;
        task.section_id = section_id;

        let mut ops = Vec::new();
        if let Some(section_op) = section_op {
            ops.push(section_op);
        }
        ops.push(new_operation(
            Uuid::new_v4(),
            OperationKind::MoveTask {
                id: task.id,
                section_id: task.section_id,
                position: task.position.clone(),
                updated_at: None,
            },
        ));
        self.apply_operations(&ops, ApplyMode::Atomic).await?;

        Ok(TaskChangeResult {
            task: self.require_task(task_id).await?,
        })
    }

    pub async fn close_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        if task.state == TaskState::Closed {
            return Ok(task);
        }
        task.state = TaskState::Closed;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateTask {
                id: task.id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: Some(task.state),
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_task(task_id).await
    }

    pub async fn open_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let mut task = self.require_task(task_id).await?;
        if task.state == TaskState::Open {
            return Ok(task);
        }
        task.state = TaskState::Open;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateTask {
                id: task.id,
                title: None,
                description: None,
                priority: None,
                tags: None,
                state: Some(task.state),
                external_identity: None,
                external_metadata: None,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_task(task_id).await
    }

    pub async fn delete_task_by_id(&self, task_id: TaskId) -> Result<Task, ServiceError> {
        let task = self.require_task(task_id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteTask { id: task_id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(task)
    }
}
