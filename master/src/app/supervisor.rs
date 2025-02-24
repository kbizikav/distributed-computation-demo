use redis::{Client, Commands, Connection, RedisResult};
use std::collections::HashSet;

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

    // アクティブなワーカーの確認
    pub fn check_active_workers(&mut self) -> RedisResult<HashSet<String>> {
        // KEYS pattern を使用（注：大規模システムではSCANを推奨）
        let worker_keys: Vec<String> = self.conn.keys("worker:heartbeat:*")?;

        let mut active_workers = HashSet::new();
        for key in worker_keys {
            // TTLが残っているキーのみを取得
            let ttl: i64 = self.conn.ttl(&key)?;
            if ttl > 0 {
                // worker:heartbeat:prefix を除去してworker_idを取得
                if let Some(worker_id) = key.strip_prefix("worker:heartbeat:") {
                    active_workers.insert(worker_id.to_string());
                }
            }
        }

        Ok(active_workers)
    }

    // 非アクティブなワーカーのクリーンアップ
    pub fn cleanup_inactive_workers(&mut self) -> RedisResult<()> {
        let active_workers = self.check_active_workers()?;

        // 全ワーカーのリストを取得（別のキーで管理されていると仮定）
        let all_workers: HashSet<String> = self.conn.smembers("workers")?;

        // 非アクティブなワーカーを特定
        for worker_id in all_workers.difference(&active_workers) {
            println!("Cleaning up inactive worker: {}", worker_id);

            // ワーカーの処理中タスクをクリーンアップ
            self.conn.hdel("processing_tasks", worker_id)?;

            // ワーカーリストから削除
            self.conn.srem("workers", worker_id)?;
        }

        Ok(())
    }
}
