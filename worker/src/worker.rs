use std::thread;
use std::time::Duration;

use common::constants::HEARTBEAT_TIMEOUT;
use common::keys::{processing_tasks_key, results_key, tasks_key};
use common::models::{Task, TaskResult};
use redis::Commands;
use redis::{Client, Connection};
use uuid::Uuid;

// Worker実装
pub struct Worker {
    pub conn: Connection,
    pub worker_id: String,
}

impl Worker {
    pub fn new(client: &Client) -> redis::RedisResult<Worker> {
        Ok(Worker {
            conn: client.get_connection()?,
            worker_id: Uuid::new_v4().to_string(),
        })
    }

    pub fn start(&mut self) -> redis::RedisResult<()> {
        loop {
            self.send_heartbeat()?;

            // ZRANGEBYSCOREコマンド: 優先度の高いタスクを1つ取得
            let tasks: Vec<String> = self.conn.zrangebyscore(tasks_key(), 0f64, "+inf")?;

            if tasks.is_empty() {
                thread::sleep(Duration::from_secs(1));
                continue;
            }

            let task: Task = serde_json::from_str(&tasks[0]).unwrap();

            // ZREMコマンド: タスクキューからタスクを削除
            self.conn.zrem::<_, _, ()>(tasks_key(), &tasks[0])?;

            // HSETコマンド: 処理中タスクとして登録
            let now = chrono::Utc::now().timestamp() as u64;
            self.conn.hset::<_, _, _, ()>(
                processing_tasks_key(),
                task.id.to_string(),
                format!("{}:{}", self.worker_id, now),
            )?;

            // タスク処理をシミュレート
            thread::sleep(Duration::from_secs(rand::random::<u64>() % 10));

            // 結果を保存
            let result = TaskResult {
                task_id: task.id,
                result: format!("Result for task {}", task.id),
            };
            let result_json = serde_json::to_string(&result).unwrap();

            // ZADDコマンド: 結果をID順に保存
            self.conn
                .zadd::<_, _, _, ()>(results_key(), result_json, task.id as f64)?;

            // HDELコマンド: 処理中タスクから削除
            self.conn
                .hdel::<_, _, ()>(processing_tasks_key(), task.id.to_string())?;

            println!("Processed task {}", task.id);
        }
    }

    pub fn send_heartbeat(&mut self) -> redis::RedisResult<()> {
        // キーの形式: "worker:heartbeat:{worker_id}"
        let key = format!("worker:heartbeat:{}", self.worker_id);

        // 方法1: SETEXを使用（アトミック）
        self.conn
            .set_ex(&key, "alive", HEARTBEAT_TIMEOUT as usize)?;

        Ok(())
    }
}
