use anyhow::Result;
use common::models::{HeartbeatRequest, Solution, TaskResponse, TaskSubmission};
use reqwest::Client;

#[derive(Clone, Debug)]
pub struct TaskClient {
    client: Client,
    base_url: String,
}

impl TaskClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn assign_task(&self) -> Result<Option<TaskResponse>> {
        let response = self
            .client
            .post(format!("{}/task/assign", self.base_url))
            .send()
            .await?;

        log::info!("Response: {:?}", response);

        match response.status() {
            reqwest::StatusCode::OK => Ok(Some(response.json().await?)),
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                let error_text = response.text().await?;
                if error_text.contains("No task available") {
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("Internal server error"))
                }
            }
            status => anyhow::bail!("Unexpected status code: {}", status),
        }
    }

    pub async fn submit_solution(&self, task_id: String, solution: Solution) -> Result<()> {
        let submission = TaskSubmission { task_id, solution };
        let response = self
            .client
            .post(format!("{}/task/submit", self.base_url))
            .json(&submission)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::NOT_FOUND => anyhow::bail!("Task not found"),
            status => anyhow::bail!("Unexpected status code: {}", status),
        }
    }

    pub async fn submit_heartbeat(&self, task_id: String, progress: f64) -> Result<()> {
        let heartbeat = HeartbeatRequest { task_id, progress };
        let response = self
            .client
            .post(format!("{}/task/heartbeat", self.base_url))
            .json(&heartbeat)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::NOT_FOUND => anyhow::bail!("Task not found"),
            reqwest::StatusCode::BAD_REQUEST => anyhow::bail!("Invalid task status"),
            status => anyhow::bail!("Unexpected status code: {}", status),
        }
    }
}
