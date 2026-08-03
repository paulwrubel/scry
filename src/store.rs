use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Task, TaskID};

#[async_trait]
pub trait TaskStore {
    async fn add(&self, description: &str) -> Result<Task, StorageError>;

    async fn complete(&self, id: TaskID) -> Result<Option<Task>, StorageError>;

    async fn delete(&self, id: TaskID) -> Result<bool, StorageError>;

    async fn list_all(&self) -> Result<Vec<Task>, StorageError>;
}

pub mod sqlite;
