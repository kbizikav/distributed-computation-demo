use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: u32,
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub x: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Result {
    pub x_squared: u32,
}
