use serde::Deserialize;

pub mod producer;

#[derive(Deserialize)]
pub struct EnvVar {
    pub redis_url: String,
}
