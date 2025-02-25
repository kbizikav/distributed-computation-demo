use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub x: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskResult {
    pub x_squared: u32,
}
