use serde::Deserialize;

pub mod api;
pub mod app;

#[derive(Deserialize)]
pub struct EnvVar {
    pub port: String,
}
