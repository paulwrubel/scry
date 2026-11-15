use crate::error::ServiceError;
use scry_core::ids::{
    Handle, MIN_HANDLE_LENGTH, TaskId, distinguishing_prefixes, shortest_handle, shortest_handles,
};
use scry_core::models::{Comment, Task};
use std::collections::HashMap;

use super::ProjectService;

impl ProjectService {
    // display handles are unique per entity type across the whole database,
    // including closed tasks and every project, so they need the complete id
    // set; display always starts at the 4-character floor
    pub async fn list_task_handles(&self) -> Result<HashMap<TaskId, String>, ServiceError> {
        let ids = self.store.get_all_task_ids(None).await?;
        Ok(shortest_handles(ids, MIN_HANDLE_LENGTH))
    }

    // display starts at the 4-character floor
    pub async fn get_task_handle(&self, id: TaskId) -> Result<String, ServiceError> {
        let ids = self.store.get_all_task_ids(None).await?;
        Ok(shortest_handle(id, ids, MIN_HANDLE_LENGTH))
    }

    pub async fn get_task_by_handle(&self, handle: &Handle) -> Result<Task, ServiceError> {
        let matches = self.store.find_task_ids_by_handle(handle, 2, None).await?;
        match matches.len() {
            0 => Err(ServiceError::TaskHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_task(matches[0]).await,
            _ => {
                let all = self
                    .store
                    .find_task_ids_by_handle(handle, i64::MAX, None)
                    .await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousTaskHandle {
                    handle: handle.to_string(),
                    candidates,
                })
            }
        }
    }

    // the current API has no comment-prefix entry point; this exists so comment
    // handles resolve like task handles wherever comment ids are later accepted
    #[allow(dead_code)]
    pub async fn get_comment_by_handle(&self, handle: &Handle) -> Result<Comment, ServiceError> {
        let matches = self
            .store
            .find_comment_ids_by_handle(handle, 2, None)
            .await?;
        match matches.len() {
            0 => Err(ServiceError::CommentHandleNotFound {
                handle: handle.to_string(),
            }),
            1 => self.require_comment(matches[0]).await,
            _ => {
                let all = self
                    .store
                    .find_comment_ids_by_handle(handle, i64::MAX, None)
                    .await?;
                let candidates = distinguishing_prefixes(&all, handle.len());
                Err(ServiceError::AmbiguousCommentHandle {
                    handle: handle.to_string(),
                    candidates,
                })
            }
        }
    }
}
