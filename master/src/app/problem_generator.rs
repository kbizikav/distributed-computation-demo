use std::{collections::HashMap, sync::Arc, time::Duration};

use common::models::{Problem, Solution};
use tokio::{sync::RwLock, time::sleep};

#[derive(Clone, Debug)]
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
        let problems = self.problems.read().await.clone();
        let mut keys = problems.keys().cloned().collect::<Vec<u64>>();
        keys.sort();
        let solutions = self.solutions.read().await;

        for x in keys {
            if !solutions.contains_key(&x) {
                let problem = problems.get(&x).unwrap();
                return Ok(Some(problem.clone()));
            }
        }
        Ok(None)
    }

    pub async fn job(self) {
        actix_web::rt::spawn(async move {
            loop {
                match self.generate_problem().await {
                    Ok(_) => {
                        log::info!("Problem generated");
                    }
                    Err(e) => {
                        log::error!("Error generating problem: {:?}", e);
                        break;
                    }
                }
                sleep(Duration::from_secs(30)).await;
            }
        });
    }
}
