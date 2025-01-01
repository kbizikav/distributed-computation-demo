use std::{error::Error, time::Duration};

use common::models::Solution;
use tokio::time::sleep;
use worker::client::TaskClient;

// 使用例
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = TaskClient::new("http://localhost:8080".to_string());

    if let Some(task) = client.assign_task().await? {
        println!("Assigned task: {:?}", task);
        let problem = task.problem;
        sleep(Duration::from_secs(3)).await;
        client.submit_heartbeat(task.id.clone(), 0.5).await?;
        let solution = Solution {
            x_squared: problem.x * problem.x,
        };
        client.submit_solution(task.id, solution).await?;
    } else {
        println!("No task available");
    }

    Ok(())
}
