use serde::Deserialize;

pub mod worker;

#[derive(Deserialize)]
pub struct EnvVar {
    pub master_server_url: String,
}
