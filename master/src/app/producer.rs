use std::thread;
use std::time::Duration;

use common::keys::{results_key, tasks_key};
use common::models::{Task, TaskResult};
use redis::Commands;
use redis::{Client, Connection};

pub struct Producer {
    conn: Connection,
    next_task_id: u32,
}

impl Producer {
    pub fn new(client: &Client) -> redis::RedisResult<Producer> {
        Ok(Producer {
            conn: client.get_connection()?,
            next_task_id: 0,
        })
    }

    pub fn create_task(&mut self, data: String) -> redis::RedisResult<u32> {
        let task = Task {
            id: self.next_task_id,
            data,
        };
        let task_json = serde_json::to_string(&task).unwrap();

        // ZADDコマンド: タスクをSorted Setに追加。スコアはタスクIDで、優先度を表す
        self.conn
            .zadd::<_, _, _, ()>(tasks_key(), task_json, task.id as f64)?;

        self.next_task_id += 1;
        Ok(task.id)
    }

    pub fn process_results(&mut self) -> redis::RedisResult<()> {
        loop {
            // ZRANGEBYSCOREコマンド: 最小スコア（最も優先度の高い）結果を取得
            let results: Vec<String> = self.conn.zrangebyscore(results_key(), 0f64, "+inf")?;

            if results.is_empty() {
                thread::sleep(Duration::from_secs(1));
                continue;
            }

            let result: TaskResult = serde_json::from_str(&results[0]).unwrap();

            // ここでDBに結果を書き込む処理を実装
            println!(
                "Processing result for task {}: {}",
                result.task_id, result.result
            );

            // ZREMコマンド: 処理済み結果を削除
            self.conn.zrem::<_, _, ()>("results", &results[0])?;
        }
    }
}
