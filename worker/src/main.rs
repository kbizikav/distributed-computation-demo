use std::{error::Error, time::Duration};

use common::models::Solution;
use tokio::time::sleep;
use worker::{client::TaskClient, EnvVar};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let env: EnvVar = envy::from_env()?;
    let client = TaskClient::new(env.master_server_url.to_string());

    println!("Starting worker");
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
