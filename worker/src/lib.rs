use serde::Deserialize;

pub mod client;
pub mod worker;

#[derive(Deserialize)]
pub struct EnvVar {
    pub master_server_url: String,
}
