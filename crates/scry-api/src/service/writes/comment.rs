use crate::error::ServiceError;
use jiff::Timestamp;
use scry_core::ids::{CommentId, ProjectId, TaskId};
use scry_core::models::Comment;
use scry_core::sync::operations::OperationKind;
use uuid::Uuid;

use super::super::{ApplyMode, ProjectService, new_operation};

impl ProjectService {
    pub async fn add_comment(
        &self,
        task_id: TaskId,
        contents: String,
    ) -> Result<Comment, ServiceError> {
        self.require_task(task_id).await?;
        let created_at = Timestamp::now();
        let comment = Comment {
            id: Uuid::new_v4(),
            task_id,
            contents,
            created_at,
            updated_at: created_at,
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateComment {
                id: comment.id,
                task_id: comment.task_id,
                contents: comment.contents.clone(),
                created_at: comment.created_at,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(comment)
    }

    pub async fn list_comments_by_project_id(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Comment>, ServiceError> {
        Ok(self
            .store
            .get_all_comments_by_project_id(project_id, None)
            .await?)
    }

    pub async fn list_comments_by_task_id(
        &self,
        task_id: TaskId,
    ) -> Result<Vec<Comment>, ServiceError> {
        Ok(self
            .store
            .get_all_comments_by_task_id(task_id, None)
            .await?)
    }

    pub async fn get_comment(&self, id: CommentId) -> Result<Option<Comment>, ServiceError> {
        Ok(self.store.get_comment_by_id(id, None).await?)
    }

    pub async fn update_comment(&self, comment: Comment) -> Result<Comment, ServiceError> {
        self.require_comment(comment.id).await?;
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateComment {
                id: comment.id,
                contents: comment.contents.clone(),
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_comment(comment.id).await
    }

    pub async fn delete_comment(&self, id: CommentId) -> Result<(), ServiceError> {
        self.require_comment(id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteComment { id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(())
    }
}
