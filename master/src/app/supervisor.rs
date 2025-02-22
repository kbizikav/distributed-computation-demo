use std::{thread, time::Duration};

use common::models::Task;
use redis::{Client, Commands, Connection};

// Supervisor実装（死活監視）
pub struct Supervisor {
    conn: Connection,
}

impl Supervisor {
    pub fn new(client: &Client) -> redis::RedisResult<Supervisor> {
        Ok(Supervisor {
            conn: client.get_connection()?,
        })
    }

    pub fn check_workers(&mut self) -> redis::RedisResult<()> {
        let heartbeat_timeout = 15; // 15秒以上heartbeatがない場合はworkerが死んでいると判断

        loop {
            let now = chrono::Utc::now().timestamp() as u64;

            // HGETALLコマンド: すべてのworker heartbeatsを取得
            let heartbeats: std::collections::HashMap<String, String> =
                self.conn.hgetall("worker_heartbeats")?;

            for (worker_id, last_heartbeat) in heartbeats {
                let last_heartbeat: u64 = last_heartbeat.parse().unwrap();
                if now - last_heartbeat > heartbeat_timeout {
                    // HDELコマンド: 死んでいるworkerを削除
                    self.conn
                        .hdel::<_, _, ()>("worker_heartbeats", &worker_id)?;

                    // HGETALLコマンド: 処理中タスクを取得
                    let processing_tasks: std::collections::HashMap<String, String> =
                        self.conn.hgetall("processing_tasks")?;

                    // 死んだworkerのタスクを再スケジュール
                    for (task_id, worker_info) in processing_tasks {
                        if worker_info.starts_with(&worker_id) {
                            let task_id: u32 = task_id.parse().unwrap();

                            // HDELコマンド: 処理中タスクから削除
                            self.conn
                                .hdel::<_, _, ()>("processing_tasks", task_id.to_string())?;

                            // ZADDコマンド: タスクを再度キューに追加
                            let task = Task {
                                id: task_id,
                                data: format!("Retried task {}", task_id),
                            };
                            let task_json = serde_json::to_string(&task).unwrap();
                            self.conn
                                .zadd::<_, _, _, ()>("tasks", task_json, task_id as f64)?;
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(5));
        }
    }

    // async fn check_workers(&mut self) -> redis::RedisResult<()> {
    //     loop {
    //         let now = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .unwrap()
    //             .as_secs();

    //         let mut conn = self.conn.clone();
    //         let heartbeats: std::collections::HashMap<String, String> = redis::cmd("HGETALL")
    //             .arg("worker_heartbeats")
    //             .query_async(&mut conn)
    //             .await?;

    //         for (worker_id, last_heartbeat) in heartbeats {
    //             let last_heartbeat: u64 = last_heartbeat.parse().unwrap();
    //             if now - last_heartbeat > 15 {
    //                 println!("Worker {} appears to be dead, cleaning up...", worker_id);

    //                 // 死んだworkerの情報を削除
    //                 redis::cmd("HDEL")
    //                     .arg("worker_heartbeats")
    //                     .arg(&worker_id)
    //                     .query_async(&mut conn)
    //                     .await?;

    //                 // 処理中タスクの再スケジュール
    //                 self.reschedule_tasks(&worker_id).await?;
    //             }
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //     }
    // }

    // async fn reschedule_tasks(&mut self, worker_id: &str) -> redis::RedisResult<()> {
    //     let mut conn = self.conn.clone();
    //     let processing_tasks: std::collections::HashMap<String, String> = redis::cmd("HGETALL")
    //         .arg("processing_tasks")
    //         .query_async(&mut conn)
    //         .await?;

    //     for (task_id, worker_info) in processing_tasks {
    //         if worker_info.starts_with(worker_id) {
    //             let task_id: u32 = task_id.parse().unwrap();

    //             // 処理中タスクから削除
    //             redis::cmd("HDEL")
    //                 .arg("processing_tasks")
    //                 .arg(task_id.to_string())
    //                 .query_async(&mut conn)
    //                 .await?;

    //             // タスクを再度キューに追加
    //             let task = Task {
    //                 id: task_id,
    //                 data: format!("Retried task {}", task_id),
    //             };
    //             let task_json = serde_json::to_string(&task).unwrap();

    //             redis::cmd("ZADD")
    //                 .arg("tasks")
    //                 .arg(task_id as f64)
    //                 .arg(task_json)
    //                 .query_async(&mut conn)
    //                 .await?;
    //         }
    //     }

    //     Ok(())
    // }
}
