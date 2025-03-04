use common::models::{Task, TaskResult};
use common::task_manager::TaskManager;
use std::sync::{Arc, Mutex};

use crate::EnvVar;

#[derive(Clone)]
pub struct Producer {
    manager: Arc<TaskManager<Task, TaskResult>>,
    results: Arc<Mutex<Vec<TaskResult>>>,
}

impl Producer {
    pub fn new(env: &EnvVar) -> anyhow::Result<Producer> {
        let manager = TaskManager::new(&env.redis_url, "task_manager", 600, 5)?;
        Ok(Producer {
            manager: Arc::new(manager),
            results: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn process_results(&self) -> anyhow::Result<()> {
        loop {
            let next_task_id = self.results.lock().unwrap().len() as u32;

            let result = match self.manager.get_result(next_task_id).await? {
                Some(result) => result,
                None => {
                    log::warn!("No result found for task {}", next_task_id);
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    continue;
                }
            };

            // store result
            self.results.lock().unwrap().push(result.clone());
            println!("Result processed for task {}", result.task_id,);

            // remove results
            self.manager.remove_result(result.task_id).await?;
        }
    }

    pub async fn run(&self) {
        // clean up
        self.manager.clear_all().await.unwrap();

        let manager = self.manager.clone();
        let supervisor_handle = tokio::spawn(async move {
            if let Err(e) = manager.cleanup_inactive_workers().await {
                eprintln!("Supervisor error: {}", e);
            }
        });

        let manager = self.manager.clone();
        let task_generator_handle = tokio::spawn(async move {
            for i in 0..100 {
                log::info!("Adding task {}", i);
                if let Err(e) = manager.add_task(i, &Task { task_id: i, x: i }).await {
                    eprintln!("Failed to create task {}: {}", i, e);
                    continue;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        let manager = self.manager.clone();
        let task_cleanup_handle = tokio::spawn(async move {
            loop {
                if let Err(e) = manager.cleanup_inactive_workers().await {
                    eprintln!("Task cleanup error: {}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let producer = self.clone();
        let process_results_handle = tokio::spawn(async move {
            if let Err(e) = producer.process_results().await {
                eprintln!("Result processor error: {}", e);
            }
        });

        log::info!("Producer started");

        tokio::try_join!(
            supervisor_handle,
            task_generator_handle,
            task_cleanup_handle,
            process_results_handle,
        )
        .unwrap();
    }
}
