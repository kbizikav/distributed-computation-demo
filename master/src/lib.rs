use serde::Deserialize;

pub mod app;

#[derive(Deserialize)]
pub struct EnvVar {
    pub port: String,
}
