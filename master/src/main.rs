use master::producer::Producer;
use std::error::Error;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env = envy::from_env::<master::EnvVar>()?;

    // Producer、Supervisorの初期化
    let producer = Producer::new(&env)?;
    producer.run().await;

    Ok(())
}
