// Data Structure in Redis:
//
// 1. Task Hash:
//    - Key: {prefix}:tasks
//    - Type: HSET
//    - Field: {task_id}
//    - Value: task_json (serialized Task object)
//    - TTL: {ttl} seconds
//
// 2. Pending Tasks:
//    - Key: {prefix}:tasks:pending
//    - Type: Set
//    - Members: {task_id}
//    - TTL: {ttl} seconds
//
// 3. Running Tasks:
//   - Key: {prefix}:tasks:running
//   - Type: Set
//   - Members: {task_id}
//   - TTL: {ttl} seconds
//
// 4. Completed Tasks:
//    - Key: {prefix}:tasks:completed
//    - Type: Set
//    - Members: {task_id}
//    - TTL: {ttl} seconds
//
// 5. Results Hash:
//    - Key: {prefix}:results
//    - Type: HSET
//    - Field: {task_id}
//    - Value: result_json (serialized TaskResult object)
//    - TTL: {ttl} seconds
//
// 6. Worker Heartbeats:
//    - Key: {prefix}:heartbeat:{task_id}
//    - Type: String
//    - Value: {worker_id}
//    - TTL: {heartbeat_ttl} seconds

use redis::{aio::Connection, AsyncCommands as _, Client};
use serde::{de::DeserializeOwned, Serialize};
type Result<T> = std::result::Result<T, TaskManagerError>;

#[derive(thiserror::Error, Debug)]
pub enum TaskManagerError {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub struct TaskManager<T: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned> {
    prefix: String,
    ttl: usize,
    heartbeat_ttl: usize,
    client: Client,
    _phantom: std::marker::PhantomData<(T, R)>,
}

impl<T: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned> TaskManager<T, R> {
    pub fn new(
        redis_url: &str,
        prefix: &str,
        ttl: usize,
        heartbeat_ttl: usize,
    ) -> Result<TaskManager<T, R>> {
        let client = Client::open(redis_url)?;
        Ok(TaskManager {
            prefix: prefix.to_owned(),
            ttl,
            heartbeat_ttl,
            client,
            _phantom: std::marker::PhantomData,
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

    pub async fn add_task(&self, task_id: u32, task: &T) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let tasks_key = format!("{}:tasks", self.prefix);
        let pending_key = format!("{}:tasks:pending", self.prefix);

        let task_json = serde_json::to_string(task)?;

        let mut pipe = redis::pipe();
        pipe.hset(&tasks_key, task_id, task_json.clone())
            .sadd(&pending_key, task_id)
            .expire(&tasks_key, self.ttl)
            .expire(&pending_key, self.ttl);

        pipe.query_async::<_, ()>(&mut conn).await?;

        Ok(())
    }

    pub async fn get_result(&self, task_id: u32) -> Result<Option<R>> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:results", self.prefix);
        let result_json: Option<String> = conn.hget(&key, task_id).await?;
        if let Some(result_json) = result_json {
            let result: R = serde_json::from_str(&result_json)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub async fn remove_old_tasks(&self, to_task_id: u32) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let tasks_key = format!("{}:tasks", self.prefix);
        let results_key = format!("{}:results", self.prefix);
        let task_ids: Vec<u32> = conn.hkeys(&tasks_key).await?;
        for task_id in task_ids {
            if task_id <= to_task_id {
                let pending_key = format!("{}:tasks:pending", self.prefix);
                let running_key = format!("{}:tasks:running", self.prefix);
                let completed_key = format!("{}:tasks:completed", self.prefix);
                conn.srem::<_, _, ()>(&pending_key, task_id).await?;
                conn.srem::<_, _, ()>(&running_key, task_id).await?;
                conn.srem::<_, _, ()>(&completed_key, task_id).await?;
                conn.hdel::<_, _, ()>(&tasks_key, task_id).await?;
                conn.hdel::<_, _, ()>(&results_key, task_id).await?;
            }
        }
        Ok(())
    }

    // // assign task to worker if available
    pub async fn assign_task(&self) -> Result<Option<(u32, T)>> {
        let mut conn = self.get_connection().await?;

        let pending_key = format!("{}:tasks:pending", self.prefix);
        // get the smallest task id
        let task_ids: Vec<u32> = redis::cmd("SORT")
            .arg(&pending_key)
            .arg("LIMIT")
            .arg(0)
            .arg(1)
            .query_async(&mut conn)
            .await?;
        let task_id = task_ids.get(0).cloned();
        if let Some(task_id) = task_id {
            let task_key = format!("{}:tasks", self.prefix);
            let task_json: String = conn.hget(&task_key, task_id).await?;
            let task: T = serde_json::from_str(&task_json)?;
            let running_key = format!("{}:tasks:running", self.prefix);
            conn.smove::<_, _, _, ()>(&pending_key, &running_key, task_id)
                .await?;
            Ok(Some((task_id, task)))
        } else {
            Ok(None)
        }
    }

    pub async fn complete_task(&self, task_id: u32, result: &R) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // add result
        let result_key = format!("{}:results", self.prefix);
        let result_json = serde_json::to_string(result)?;
        conn.hset::<_, _, _, ()>(&result_key, task_id, result_json)
            .await?;

        // move task from running to completed
        let running_key = format!("{}:tasks:running", self.prefix);
        let completed_key = format!("{}:tasks:completed", self.prefix);
        conn.smove::<_, _, _, ()>(&running_key, &completed_key, task_id)
            .await?;

        // set expiration
        conn.expire::<_, ()>(&result_key, self.ttl).await?;

        Ok(())
    }

    pub async fn submit_heartbeat(&self, worker_id: &str, task_id: u32) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:heartbeat:{}", self.prefix, task_id);
        conn.set_ex::<_, _, ()>(&key, worker_id, self.heartbeat_ttl)
            .await?;
        Ok(())
    }

    pub async fn cleanup_inactive_tasks(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // get all running tasks
        let running_key = format!("{}:tasks:running", self.prefix);
        let pending_key = format!("{}:tasks:pending", self.prefix);
        let task_ids: Vec<u32> = conn.smembers(&running_key).await?;

        for task_id in task_ids {
            let key = format!("{}:heartbeat:{}", self.prefix, task_id);
            let worker_id: Option<String> = conn.get(&key).await?;
            if worker_id.is_none() {
                // move task from running to pending
                conn.smove::<_, _, _, ()>(&running_key, &pending_key, task_id)
                    .await?;
            }
        }
        Ok(())
    }
}
