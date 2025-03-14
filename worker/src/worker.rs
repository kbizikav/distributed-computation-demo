use common::models::{Task, TaskResult};
use common::task_manager::TaskManager;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::EnvVar;

#[derive(Clone)]
pub struct Worker {
    pub worker_id: String,
    pub manager: Arc<TaskManager<Task, TaskResult>>,
    pub task_ids: Arc<Mutex<HashSet<u32>>>,
}

const HEARTBEAT_INTERVAL: usize = 10;

impl Worker {
    pub fn new(env: &EnvVar) -> anyhow::Result<Worker> {
        let worker_id = Uuid::new_v4().to_string();
        let manager =
            TaskManager::new(&env.redis_url, "task_manager", 600, HEARTBEAT_INTERVAL * 3)?;
        Ok(Worker {
            worker_id,
            manager: Arc::new(manager),
            task_ids: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub async fn solve(&self) -> anyhow::Result<()> {
        loop {
            let task = self.manager.assign_task().await?;

            if task.is_none() {
                thread::sleep(Duration::from_secs(1));
                log::info!("No task assigned");
                continue;
            }
            let (task_id, task) = task.unwrap();

            self.task_ids.lock().await.insert(task_id);
            log::info!("Processing task {}", task.task_id);

            thread::sleep(Duration::from_secs(rand::random::<u64>() % 20));

            let result = TaskResult {
                task_id: task.task_id,
                x_squared: task.x * task.x,
            };

            self.manager.complete_task(task_id, &result).await?;
            self.task_ids.lock().await.remove(&task_id);
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

        let worker = self.clone();
        let submit_heartbeat_handle = tokio::spawn(async move {
            loop {
                log::info!("Submitting heartbeat");
                let task_ids = worker.task_ids.lock().await.clone();
                for task_id in task_ids.iter() {
                    if let Err(e) = worker
                        .manager
                        .submit_heartbeat(&worker.worker_id, *task_id)
                        .await
                    {
                        eprintln!("Error: {:?}", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_INTERVAL as u64))
                    .await;
            }
        });

        tokio::try_join!(solve_handle, submit_heartbeat_handle).unwrap();
    }
}
