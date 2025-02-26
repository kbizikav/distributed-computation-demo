use common::models::TaskResult;
use common::task_manager::TaskManager;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use crate::EnvVar;

#[derive(Clone)]
pub struct Worker {
    pub worker_id: String,
    pub manager: Arc<TaskManager>,
}

impl Worker {
    pub fn new(env: &EnvVar) -> anyhow::Result<Worker> {
        let worker_id = Uuid::new_v4().to_string();
        let manager = TaskManager::new(&env.redis_url, "task_manager", 600, 10)?;
        Ok(Worker {
            worker_id,
            manager: Arc::new(manager),
        })
    }

    pub async fn solve(&self) -> anyhow::Result<()> {
        loop {
            let task = self.manager.assign_task(&self.worker_id).await?;

            if task.is_none() {
                thread::sleep(Duration::from_secs(1));
                log::info!("No task assigned");
                continue;
            }
            let (task_id, task) = task.unwrap();

            log::info!("Processing task {}", task.task_id);

            thread::sleep(Duration::from_secs(rand::random::<u64>() % 10));

            let result = TaskResult {
                task_id: task.task_id,
                x_squared: task.x * task.x,
            };

            self.manager
                .complete_task(&self.worker_id, task_id, &result)
                .await?;
            println!("Processed task {}", task.task_id);
        }
    }

    pub async fn run(&self) {
        let worker = self.clone();
        let solve_handle = tokio::spawn(async move {
            log::info!("Starting worker");
            if let Err(e) = worker.solve().await {
                eprintln!("Error: {:?}", e);
            }
        });

        let manager = self.manager.clone();
        let worker_id = self.worker_id.clone();
        let submit_heartbeat_handle = tokio::spawn(async move {
            loop {
                log::info!("Submitting heartbeat");
                if let Err(e) = manager.submit_heartbeat(&worker_id).await {
                    eprintln!("Error: {:?}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });

        tokio::try_join!(solve_handle, submit_heartbeat_handle).unwrap();
    }
}
