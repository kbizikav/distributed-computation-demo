use master::app::{producer::Producer, supervisor::Supervisor};
use redis::Client;
use std::error::Error;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Redisクライアントの初期化
    let client = Client::open("redis://127.0.0.1:6379")?;
    let task_count = 100;

    // Producer、Supervisorの初期化
    let mut producer = Producer::new(&client)?;
    let mut supervisor = Supervisor::new(&client)?;

    // Supervisorを別タスクで実行
    let supervisor_handle = tokio::spawn(async move {
        if let Err(e) = supervisor.check_workers() {
            eprintln!("Supervisor error: {}", e);
        }
    });

    // タスク生成用タスク
    let task_generator_handle = tokio::spawn(async move {
        for i in 0..task_count {
            if let Err(e) = producer.create_task(format!("Task data {}", i)) {
                eprintln!("Failed to create task {}: {}", i, e);
                continue;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // 結果処理用タスク
    let mut result_processor = Producer::new(&client)?;
    let result_processor_handle = tokio::spawn(async move {
        if let Err(e) = result_processor.process_results() {
            eprintln!("Result processor error: {}", e);
        }
    });

    // 全てのタスクの完了を待つ
    tokio::try_join!(
        supervisor_handle,
        task_generator_handle,
        result_processor_handle,
    )?;

    Ok(())
}
