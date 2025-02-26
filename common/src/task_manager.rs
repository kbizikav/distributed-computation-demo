// Data Structure in Redis:
//
// 1. Tasks Queue:
//    - Key: {prefix}:tasks
//    - Type: Sorted Set
//    - Members: task_json (serialized Task objects)
//    - Scores: task_id
//    - TTL: {ttl} seconds
//
// 2. Worker Assigned Tasks:
//    - Key: {prefix}:worker:{worker_id}
//    - Type: Sorted Set
//    - Members: task_json (serialized Task objects)
//    - Scores: task_id
//    - TTL: {ttl} seconds
//
// 3. Task Results:
//    - Key: {prefix}:result:{task_id}
//    - Type: String
//    - Value: result_json (serialized TaskResult object)
//    - TTL: {ttl} seconds
//
// 4. Worker Heartbeats:
//    - Key: {prefix}:heartbeat:{worker_id}
//    - Type: String
//    - Value: "" (empty string)
//    - TTL: {heartbeat_ttl} seconds
//
// Flow:
// - Tasks are initially added to the tasks queue
// - Workers request tasks which are moved from tasks queue to worker's task list
// - When tasks are completed, they are removed from worker's list and results are stored
// - Workers send heartbeats to indicate they are still active
// - Inactive workers have their tasks re-queued back to the tasks queue

use crate::models::{Task, TaskResult};
use redis::{aio::Connection, AsyncCommands as _, Client};
type Result<T> = std::result::Result<T, TaskManagerError>;

#[derive(thiserror::Error, Debug)]
pub enum TaskManagerError {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub struct TaskManager {
    prefix: String,
    ttl: usize,
    heartbeat_ttl: usize,
    client: Client,
}

impl TaskManager {
    pub fn new(
        redis_url: &str,
        prefix: &str,
        ttl: usize,
        heartbeat_ttl: usize,
    ) -> Result<TaskManager> {
        let client = Client::open(redis_url)?;
        Ok(TaskManager {
            prefix: prefix.to_owned(),
            ttl,
            heartbeat_ttl,
            client,
        })
    }

    async fn get_connection(&self) -> Result<Connection> {
        Ok(self.client.get_async_connection().await?)
    }

    pub async fn clear_all(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let keys: Vec<String> = conn.keys(format!("{}:*", self.prefix)).await?;
        for key in keys {
            conn.del::<_, ()>(key).await?;
        }
        Ok(())
    }

    pub async fn add_task(&self, task_id: u32, task: &Task) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let key = format!("{}:tasks", self.prefix);
        let member = serde_json::to_string(task)?;
        conn.zadd::<_, _, _, ()>(&key, member, task_id as f64)
            .await?;

        // set expiration
        conn.expire::<_, ()>(&key, self.ttl).await?;

        Ok(())
    }

    pub async fn get_result(&self, task_id: u32) -> Result<Option<TaskResult>> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:result:{}", self.prefix, task_id);

        let exists: bool = conn.exists(&key).await?;
        if !exists {
            return Ok(None);
        }

        let result_json: String = conn.get(&key).await?;
        Ok(Some(serde_json::from_str(&result_json)?))
    }

    pub async fn remove_result(&self, task_id: u32) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:result:{}", self.prefix, task_id);
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    // assign task to worker if available
    pub async fn assign_task(&self, worker_id: &str) -> Result<Option<(u32, Task)>> {
        let mut conn = self.get_connection().await?;

        let task_key = format!("{}:tasks", self.prefix);

        // get task from sorted set
        let task: Option<(String, f64)> = conn
            .zpopmin::<_, Vec<(String, f64)>>(&task_key, 1)
            .await?
            .into_iter()
            .next();

        if let Some((task_json, task_id)) = task {
            // add task to worker's list
            let task: Task = serde_json::from_str(&task_json)?;
            let key = format!("{}:worker:{}", self.prefix, worker_id);
            let member = serde_json::to_string(&task)?;
            conn.zadd::<_, _, _, ()>(&key, member, task_id).await?;

            // set expiration
            conn.expire::<_, ()>(&key, self.ttl).await?;

            // remove task from tasks list
            conn.zrem::<_, _, ()>(&task_key, task_json).await?;

            Ok(Some((task_id as u32, task)))
        } else {
            Ok(None)
        }
    }

    pub async fn complete_task(
        &self,
        worker_id: &str,
        task_id: u32,
        task: &Task,
        result: &TaskResult,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // remove task from worker's list
        let worker_key = format!("{}:worker:{}", self.prefix, worker_id);
        let task_json = serde_json::to_string(task)?;
        conn.zrem::<_, _, ()>(worker_key, task_json).await?;

        // add result
        let result_key = format!("{}:result:{}", self.prefix, task_id);
        let result_json = serde_json::to_string(result)?;
        conn.set::<_, _, ()>(&result_key, result_json).await?;

        // set expiration
        conn.expire::<_, ()>(&result_key, self.ttl).await?;

        Ok(())
    }

    pub async fn submit_heartbeat(&self, worker_id: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let key = format!("{}:heartbeat:{}", self.prefix, worker_id);
        conn.set::<_, _, ()>(&key, "").await?;

        // set expiration
        conn.expire::<_, ()>(&key, self.heartbeat_ttl).await?;

        Ok(())
    }

    // remove inactive workers and re-queue their tasks
    pub async fn cleanup_inactive_workers(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let worker_ids: Vec<String> = conn
            .keys::<_, Vec<String>>(format!("{}:worker:*", self.prefix))
            .await?
            .into_iter()
            .map(|key| key.split(':').last().unwrap().to_string())
            .collect();

        for worker_id in worker_ids {
            let key = format!("{}:heartbeat:{}", self.prefix, worker_id);
            let ttl: i64 = conn.ttl(&key).await?;
            if ttl < 0 {
                // re-queue tasks
                let worker_key = format!("{}:worker:{}", self.prefix, worker_id);
                let tasks: Vec<(String, f64)> = conn
                    .zrangebyscore_withscores(&worker_key, 0.0, "+inf")
                    .await?;
                for (task_json, task_id) in tasks {
                    let key = format!("{}:tasks", self.prefix);
                    conn.zadd::<_, _, _, ()>(&key, task_json, task_id).await?;

                    // set expiration
                    conn.expire::<_, ()>(&key, self.ttl).await?;

                    log::info!("Re-queued task {} from worker {}", task_id, worker_id);
                }

                // remove worker
                conn.del::<_, ()>(worker_key).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use uuid::Uuid;

    use super::*;
    use crate::models::Task;

    #[tokio::test]
    async fn test_add_task() {
        let task_manager = TaskManager::new("redis://localhost:6379", "test", 60, 3)
            .expect("Failed to create TaskManager");

        task_manager.clear_all().await.unwrap();

        for i in 0..10 {
            let task = Task { task_id: i, x: i };
            task_manager.add_task(task.task_id, &task).await.unwrap();
        }

        let worker_id = Uuid::new_v4().to_string();
        for _ in 0..10 {
            let task = task_manager.assign_task(&worker_id).await.unwrap();

            let task: Task = task.unwrap().1;
            let task_result = TaskResult {
                task_id: task.task_id,
                x_squared: task.x * task.x,
            };
            task_manager
                .complete_task(&worker_id, task.task_id, &task, &task_result)
                .await
                .unwrap();
        }

        for i in 0..10 {
            let result = task_manager.get_result(i).await.unwrap();
            let result = result.unwrap();
            assert_eq!(result.task_id, i);
            assert_eq!(result.x_squared, i * i);
        }
    }
}
