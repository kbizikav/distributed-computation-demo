use master::producer::Producer;
use std::error::Error;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let env = envy::from_env::<master::EnvVar>()?;

    // Producer、Supervisorの初期化
    let producer = Producer::new(&env)?;
    producer.run().await;

    Ok(())
}
