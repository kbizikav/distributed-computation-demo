use actix_web::{
    post,
    web::{Data, Json},
    Error,
};
use common::models::{HeartbeatRequest, TaskResponse, TaskSubmission};

use crate::app::task_manager::TaskManager;

#[post("/assign")]
pub async fn assign_task(task_manager: Data<TaskManager>) -> Result<Json<TaskResponse>, Error> {
    task_manager
        .assign_task()
        .await
        .map(|opt_task| match opt_task {
            Some(task) => {
                let response = TaskResponse {
                    id: task.id,
                    problem: task.problem,
                };
                Ok(Json(response))
            }
            None => Err(actix_web::error::ErrorInternalServerError(
                "No task available",
            )),
        })
        .map_err(|_| actix_web::error::ErrorInternalServerError("Internal server error"))?
}

#[post("/submit")]
pub async fn submit_task(
    task_manager: Data<TaskManager>,
    submission: Json<TaskSubmission>,
) -> Result<Json<()>, Error> {
    task_manager
        .submit_task(&submission.task_id, &submission.solution)
        .await
        .map(|_| Json(()))
        .map_err(|_| actix_web::error::ErrorNotFound("Task not found"))
}

#[post("/heartbeat")]
pub async fn submit_heartbeat(
    task_manager: Data<TaskManager>,
    heartbeat: Json<HeartbeatRequest>,
) -> Result<Json<()>, Error> {
    task_manager
        .submit_heartbeat(&heartbeat.task_id, heartbeat.progress)
        .await
        .map(|_| Json(()))
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("Task not found") {
                actix_web::error::ErrorNotFound("Task not found")
            } else if error_msg.contains("Task is not assigned") {
                actix_web::error::ErrorBadRequest("Invalid task status")
            } else {
                actix_web::error::ErrorInternalServerError("Internal server error")
            }
        })
}

pub fn task_scope() -> actix_web::Scope {
    actix_web::web::scope("/task")
        .service(assign_task)
        .service(submit_task)
        .service(submit_heartbeat)
}
