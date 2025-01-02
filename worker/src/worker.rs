use std::{sync::Arc, time::Duration};

use crate::client::TaskClient;
use common::{
    constants::HEART_BEAT_INTERVAL,
    models::{Problem, Solution},
};
use tokio::{sync::RwLock, time::sleep};

#[derive(Clone, Debug)]
pub struct TaskProgress {
    pub task_id: String,
    pub progress: f64,
    pub problem: Problem,
    pub solution: Option<Solution>,
}

#[derive(Clone, Debug)]
pub struct Worker {
    client: TaskClient,
    pub task: Arc<RwLock<Option<TaskProgress>>>,
}

impl Worker {
    pub fn new(master_server_url: String) -> Self {
        Self {
            client: TaskClient::new(master_server_url),
            task: Arc::new(RwLock::new(None)),
        }
    }

    async fn solve_task(&self) {
        if self.task.read().await.is_none() {
            let task = self.client.assign_task().await.unwrap();
            if let Some(task) = task {
                *self.task.write().await = Some(TaskProgress {
                    task_id: task.id.clone(),
                    progress: 0.0,
                    problem: task.problem,
                    solution: None,
                });
            }
        }
        let task = self.task.read().await.clone().unwrap();
        sleep(Duration::from_secs(10)).await; // Simulate work
        *self.task.write().await = Some(TaskProgress {
            task_id: task.task_id.clone(),
            progress: 0.5,
            problem: task.problem.clone(),
            solution: None,
        }); // update progress
        sleep(Duration::from_secs(10)).await; // Simulate work
        let solution = Solution {
            x_squared: task.problem.x * task.problem.x,
        };
        *self.task.write().await = Some(TaskProgress {
            task_id: task.task_id.clone(),
            progress: 1.0,
            problem: task.problem.clone(),
            solution: Some(solution),
        });
    }

    async fn solve_task_job(self) {
        tokio::spawn(async move {
            loop {
                self.solve_task().await;
                sleep(Duration::from_secs(10)).await;
            }
        });
    }

    async fn submit(&self) {
        let task = self.task.read().await.clone();
        if let Some(task) = task {
            if let Some(solution) = task.solution {
                // submit solution if task is completed
                self.client
                    .submit_solution(task.task_id.clone(), solution)
                    .await
                    .unwrap();
                *self.task.write().await = None;
            } else {
                // submit heartbeat if task is in progress
                self.client
                    .submit_heartbeat(task.task_id.clone(), task.progress)
                    .await
                    .unwrap();
            }
        }
    }

    async fn submit_job(self) {
        tokio::spawn(async move {
            loop {
                self.submit().await;
                sleep(Duration::from_secs(HEART_BEAT_INTERVAL)).await;
            }
        });
    }

    pub async fn job(&self) {
        self.clone().solve_task_job().await;
        self.clone().submit_job().await;
    }
}
