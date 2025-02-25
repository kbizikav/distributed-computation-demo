use serde::Deserialize;

pub mod worker;

#[derive(Deserialize)]
pub struct EnvVar {
    pub redis_url: String,
}
