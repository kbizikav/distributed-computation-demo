use redis::Client;
use tokio;
use worker::worker::Worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Redisクライアントの初期化
    let client = Client::open("redis://127.0.0.1:6379")?;

    // Workerの初期化
    let mut worker = Worker::new(&client)?;

    // heartbeat送信用タスク
    let worker_id = worker.worker_id.clone();
    let heartbeat_client = client.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut heartbeat_worker = Worker {
            conn: heartbeat_client.get_connection().unwrap(),
            worker_id,
        };

        loop {
            if let Err(e) = heartbeat_worker.send_heartbeat() {
                eprintln!("Heartbeat error: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // メインのタスク処理ループ
    let worker_handle = tokio::spawn(async move {
        if let Err(e) = worker.start() {
            eprintln!("Worker error: {}", e);
        }
    });

    // 全てのタスクの完了を待つ
    tokio::try_join!(heartbeat_handle, worker_handle)?;

    Ok(())
}
