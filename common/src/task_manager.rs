use redis::{aio::Connection, AsyncCommands as _, Client};

use crate::models::{Task, TaskResult};

type Result<T> = std::result::Result<T, TaskManagerError>;

#[derive(thiserror::Error, Debug)]
pub enum TaskManagerError {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub struct TaskManager {
    client: Client,
}

impl TaskManager {
    pub fn new(redis_url: &str) -> Result<TaskManager> {
        let client = Client::open(redis_url)?;
        Ok(TaskManager { client })
    }

    async fn get_connection(&self) -> Result<Connection> {
        Ok(self.client.get_async_connection().await?)
    }

    pub async fn add_task(&self, task_id: u32, task: &Task) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let member = serde_json::to_string(task)?;
        conn.zadd::<_, _, _, ()>("tasks", member, task_id as f64)
            .await?;

        Ok(())
    }

    // assign task to worker if available
    pub async fn assign_task(&self, worker_id: &str) -> Result<Option<(u32, Task)>> {
        let mut conn = self.get_connection().await?;

        // get task from sorted set
        let task: Option<(String, f64)> = conn
            .zpopmin::<&str, Vec<(String, f64)>>("tasks", 1)
            .await?
            .into_iter()
            .next();

        if let Some((task_json, task_id)) = task {
            // add task to worker's list
            let task: Task = serde_json::from_str(&task_json)?;
            let key = format!("worker:{}", worker_id);
            let member = serde_json::to_string(&task)?;
            conn.zadd::<_, _, _, ()>(key, member, task_id).await?;
            Ok(Some((task_id as u32, task)))
        } else {
            Ok(None)
        }
    }

    pub async fn complete_task(
        &self,
        worker_id: &str,
        task_id: u32,
        result: &TaskResult,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // remove task from worker's list
        let key = format!("worker:{}", worker_id);
        conn.zrem::<_, _, ()>(key, task_id).await?;

        // add result to sorted set
        let member = serde_json::to_string(result)?;
        conn.zadd::<_, _, _, ()>("results", member, task_id as f64)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::Task;

    #[tokio::test]
    async fn test_add_task() {
        let task_manager = TaskManager::new("redis://localhost:6379").unwrap();
        let task = Task { x: 42 };
        task_manager.add_task(1, &task).await.unwrap();
    }
}
