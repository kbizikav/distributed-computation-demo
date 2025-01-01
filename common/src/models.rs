use serde::{Deserialize, Serialize};

// A problem to calculate x^2 for the given x
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Problem {
    pub x: u64,
}

// A solution to the problem
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Solution {
    pub x_squared: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub task_id: String,
    pub progress: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskSubmission {
    pub task_id: String,
    pub x_squared: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub problem: Problem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApiError {
    TaskNotFound,
    InvalidTaskStatus,
    InternalError,
}
