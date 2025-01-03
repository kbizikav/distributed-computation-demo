use std::{collections::HashMap, sync::Arc};

use common::{
    constants::HEART_BEAT_TIMEOUT,
    models::{Problem, Solution},
};
use tokio::sync::RwLock;

use super::{problem_generator::ProblemGenerator, utils::get_unix_timestamp};

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned { last_heartbeat: u64, progress: f64 },
    Completed,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub problem: Problem,
    pub status: TaskStatus,
}

#[derive(Clone, Debug)]
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
                problem,
                status: TaskStatus::Assigned {
                    last_heartbeat: get_unix_timestamp(),
                    progress: 0.0,
                },
            };
            let mut tasks = self.tasks.write().await;
            tasks.insert(id, task.clone());
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    pub async fn submit_task(&self, id: &str, solution: &Solution) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            self.problem_generator
                .register_solution(&task.problem, solution)
                .await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub async fn submit_heartbeat(&self, id: &str, progress_: f64) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            if let TaskStatus::Assigned {
                last_heartbeat,
                progress,
            } = &mut task.status
            {
                *last_heartbeat = get_unix_timestamp();
                *progress = progress_;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Task is not assigned"))
            }
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub async fn cleanup_tasks(&self) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut to_remove = vec![];
        for (id, task) in tasks.iter() {
            if let TaskStatus::Assigned { last_heartbeat, .. } = task.status {
                if get_unix_timestamp() - last_heartbeat > HEART_BEAT_TIMEOUT {
                    to_remove.push(id.clone());
                }
            }
        }
        for id in to_remove {
            log::warn!("Task {} is removed due to heartbeat timeout", id);
            tasks.get_mut(&id).unwrap().status = TaskStatus::Pending;
        }
        Ok(())
    }

    pub async fn get_num_unsolved_problems(&self) -> anyhow::Result<usize> {
        let problems = self.problem_generator.problems.read().await;
        let solutions = self.problem_generator.solutions.read().await;
        Ok(problems.len() - solutions.len())
    }

    pub async fn clean_up_job(self) {
        actix_web::rt::spawn(async move {
            loop {
                match self.cleanup_tasks().await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error cleaning up tasks: {:?}", e);
                        break;
                    }
                }
                log::info!(
                    "Unsolved problems: {}",
                    self.get_num_unsolved_problems().await.unwrap()
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    pub async fn job(self) -> anyhow::Result<()> {
        let problem_generator = self.problem_generator.clone();
        problem_generator.job().await;
        self.clean_up_job().await;
        Ok(())
    }
}
