use worker::worker::Worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let env = envy::from_env::<worker::EnvVar>()?;
    let worker = Worker::new(&env)?;
    worker.run().await;
    Ok(())
}
