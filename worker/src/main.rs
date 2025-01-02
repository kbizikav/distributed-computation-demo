use std::{error::Error, time::Duration};

use log::info;
use worker::{worker::Worker, EnvVar};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let env: EnvVar = envy::from_env()?;
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let worker = Worker::new(env.master_server_url);
    worker.job().await;
    loop {
        let task = worker.task.read().await;
        info!("Worker status: {:?}", *task);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
