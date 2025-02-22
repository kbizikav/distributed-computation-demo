use std::thread;
use std::time::Duration;

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
            .zadd::<_, _, _, ()>("tasks", task_json, self.next_task_id as f64)?;

        self.next_task_id += 1;
        Ok(task.id)
    }

    pub fn process_results(&mut self) -> redis::RedisResult<()> {
        loop {
            // ZRANGEBYSCOREコマンド: 最小スコア（最も優先度の高い）結果を取得
            let results: Vec<String> = self.conn.zrangebyscore("results", 0f64, "+inf")?;

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

    // pub async fn create_task(&mut self, data: String) -> redis::RedisResult<u32> {
    //     let task = Task {
    //         id: self.next_task_id,
    //         data,
    //     };
    //     let task_json = serde_json::to_string(&task).unwrap();

    //     redis::cmd("ZADD")
    //         .arg("tasks")
    //         .arg(self.next_task_id as f64)
    //         .arg(task_json)
    //         .query_async(&mut self.conn)
    //         .await?;

    //     self.next_task_id += 1;
    //     Ok(task.id)
    // }

    // async fn process_results(&mut self) -> redis::RedisResult<()> {
    //     loop {
    //         let mut conn = self.conn.clone();
    //         let results: Vec<String> = redis::cmd("ZRANGEBYSCORE")
    //             .arg("results")
    //             .arg(0f64)
    //             .arg("+inf")
    //             .arg("LIMIT")
    //             .arg(0)
    //             .arg(1)
    //             .query_async(&mut conn)
    //             .await?;

    //         if results.is_empty() {
    //             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //             continue;
    //         }

    //         let result: TaskResult = serde_json::from_str(&results[0]).unwrap();

    //         // DBへの書き込み処理（非同期で実装）
    //         println!(
    //             "Processing result for task {}: {}",
    //             result.task_id, result.result
    //         );

    //         redis::cmd("ZREM")
    //             .arg("results")
    //             .arg(&results[0])
    //             .query_async(&mut conn)
    //             .await?;
    //     }
    // }
}
