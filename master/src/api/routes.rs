use actix_web::{
    get, post,
    web::{Data, Json},
    HttpResponse, Result,
};
use common::models::{ApiError, HeartbeatRequest, TaskResponse, TaskSubmission};

use crate::app::task_manager::TaskManager;

#[get("/task")]
async fn get_task(task_manager: Data<TaskManager>) -> Result<HttpResponse> {
    match task_manager.assign_task().await {
        Ok(Some(task)) => {
            let response = TaskResponse {
                id: task.id,
                problem: task.problem,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NoContent().finish()),
        Err(_) => Ok(HttpResponse::InternalServerError().json(ApiError::InternalError)),
    }
}

#[post("/task/submit")]
async fn submit_task(
    task_manager: Data<TaskManager>,
    submission: Json<TaskSubmission>,
) -> Result<HttpResponse> {
    match task_manager
        .submit_task(&submission.task_id, submission.x_squared)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::NotFound().json(ApiError::TaskNotFound)),
    }
}

#[post("/task/heartbeat")]
async fn submit_heartbeat(
    task_manager: Data<TaskManager>,
    heartbeat: Json<HeartbeatRequest>,
) -> Result<HttpResponse> {
    match task_manager
        .submit_heartbeat(&heartbeat.task_id, heartbeat.progress)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            if e.to_string().contains("Task not found") {
                Ok(HttpResponse::NotFound().json(ApiError::TaskNotFound))
            } else if e.to_string().contains("Task is not assigned") {
                Ok(HttpResponse::BadRequest().json(ApiError::InvalidTaskStatus))
            } else {
                Ok(HttpResponse::InternalServerError().json(ApiError::InternalError))
            }
        }
    }
}
