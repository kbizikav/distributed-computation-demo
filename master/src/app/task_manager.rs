use std::{collections::HashMap, sync::Arc};

use common::{constants::HEART_BEAT_TIMEOUT, models::Problem};
use tokio::sync::RwLock;

use super::{problem_generator::ProblemGenerator, utils::get_unix_timestamp};

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub last_heartbeat: u64,
    pub problem: Problem,
    pub status: TaskStatus,
}

pub struct TaskManager {
    pub problem_generator: ProblemGenerator,
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskManager {
    pub async fn new(problem_generator: ProblemGenerator) -> anyhow::Result<Self> {
        Ok(TaskManager {
            problem_generator,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn assign_task(&self) -> anyhow::Result<Option<Task>> {
        let problem = self.problem_generator.get_unsolved_problem().await?;
        if let Some(problem) = problem {
            let id = uuid::Uuid::new_v4().to_string();
            let task = Task {
                id: id.clone(),
                last_heartbeat: 0,
                problem,
                status: TaskStatus::Pending,
            };
            let mut tasks = self.tasks.write().await;
            tasks.insert(id, task.clone());
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    pub async fn submit_task(&self, id: &str, x_squared: u64) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            self.problem_generator
                .register_solution(task.problem.x, x_squared)
                .await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub async fn submit_heartbeat(&self, id: &str) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.last_heartbeat = get_unix_timestamp();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub async fn cleanup_tasks(&self) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        let current_timestamp = get_unix_timestamp();
        let mut tasks_to_remove = vec![];
        for (id, task) in tasks.iter() {
            if current_timestamp - task.last_heartbeat > HEART_BEAT_TIMEOUT {
                tasks_to_remove.push(id.clone());
            }
        }

        // Remove the tasks
        for id in tasks_to_remove {
            tasks.remove(&id);
        }
        Ok(())
    }
}
