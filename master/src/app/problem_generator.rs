use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

// A problem to calculate x^2 for the given x
#[derive(Clone, Debug)]
pub struct Problem {
    pub x: u64,
    pub x_squared: Option<u64>,
}

pub struct ProblemGenerator {
    pub problems: Arc<RwLock<HashMap<u64, Problem>>>,
}

impl ProblemGenerator {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(ProblemGenerator {
            problems: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn generate_problem(&self) -> anyhow::Result<()> {
        let mut problems = self.problems.write().await;
        let x = problems.len() as u64;
        let problem = Problem { x, x_squared: None };
        problems.insert(x, problem);
        Ok(())
    }

    pub async fn register_solution(&self, x: u64, x_squared: u64) -> anyhow::Result<()> {
        let mut problems = self.problems.write().await;
        if let Some(problem) = problems.get_mut(&x) {
            problem.x_squared = Some(x_squared);
        } else {
            return Err(anyhow::anyhow!("Problem not found"));
        }
        Ok(())
    }

    pub async fn get_problem(&self) -> anyhow::Result<Option<Problem>> {
        let problems = self.problems.read().await;
        let problem = problems
            .values()
            .find(|problem| problem.x_squared.is_none())
            .cloned();
        Ok(problem)
    }
}
