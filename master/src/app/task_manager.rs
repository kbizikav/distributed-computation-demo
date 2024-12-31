use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use super::problem_generator::{Problem, ProblemGenerator};

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub last_heartbeat_at: u64,
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
        let problem = self.problem_generator.get_problem().await?;
        if let Some(problem) = problem {
            let id = uuid::Uuid::new_v4().to_string();
            let task = Task {
                id: id.clone(),
                last_heartbeat_at: 0,
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
            task.problem.x_squared = Some(x_squared);
            task.status = TaskStatus::Completed;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub async fn submit_heartbeat(&self, id: &str) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Pending;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }


}
