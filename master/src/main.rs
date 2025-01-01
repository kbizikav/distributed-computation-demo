use actix_web::{
    middleware::Logger,
    web::{Data, JsonConfig},
    App, HttpServer,
};
use master::{
    api::routes::task_scope,
    app::{problem_generator::ProblemGenerator, task_manager::TaskManager},
    EnvVar,
};
use std::io::{self};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let env: EnvVar = envy::from_env().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse environment variables: {}", e),
        )
    })?;

    log::info!("Starting master server");

    let problem_generator = ProblemGenerator::new().await.unwrap();
    let task_manager = TaskManager::new(problem_generator).await.unwrap();

    let task_manager_for_job = task_manager.clone();

    log::info!("Starting task manager job");
    task_manager_for_job.job().await.unwrap();

    log::info!("Starting http server");
    let data = Data::new(task_manager);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(JsonConfig::default().limit(35_000_000))
            .app_data(data.clone())
            .service(task_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
