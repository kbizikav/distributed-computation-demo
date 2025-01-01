use serde::Deserialize;

pub mod client;

#[derive(Deserialize)]
pub struct EnvVar {
    pub master_server_url: String,
}
