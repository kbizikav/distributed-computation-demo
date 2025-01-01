use std::{collections::HashMap, sync::Arc};

use common::models::{Problem, Solution};
use tokio::sync::RwLock;

pub struct ProblemGenerator {
    pub problems: Arc<RwLock<HashMap<u64, Problem>>>,
    pub solutions: Arc<RwLock<HashMap<u64, Solution>>>,
}

impl ProblemGenerator {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(ProblemGenerator {
            problems: Arc::new(RwLock::new(HashMap::new())),
            solutions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn generate_problem(&self) -> anyhow::Result<()> {
        let mut problems = self.problems.write().await;
        let x = problems.len() as u64;
        let problem = Problem { x };
        problems.insert(x, problem);
        Ok(())
    }

    pub async fn register_solution(
        &self,
        problem: &Problem,
        solution: &Solution,
    ) -> anyhow::Result<()> {
        let x = problem.x;
        let is_problem_exist = self.problems.read().await.contains_key(&x);
        if !is_problem_exist {
            return Err(anyhow::anyhow!("Problem not found"));
        }
        self.solutions.write().await.insert(x, solution.clone());
        Ok(())
    }

    pub async fn get_unsolved_problem(&self) -> anyhow::Result<Option<Problem>> {
        let problems = self.problems.read().await;
        let solutions = self.solutions.read().await;
        for (x, problem) in problems.iter() {
            if !solutions.contains_key(x) {
                return Ok(Some(problem.clone()));
            }
        }
        Ok(None)
    }
}
