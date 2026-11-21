use serde::{Deserialize, Serialize};
use std::error::Error;
use vermouth_common::fetch_container;

pub use vermouth_common::Container;
#[derive(Serialize, Deserialize, Clone)]
pub struct NanoKVM {
    pub server: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub manual: bool,
    pub wait_time: u64,
    pub nanokvm: NanoKVM,
}

pub async fn load() -> Result<Config, Box<dyn Error>> {
    vermouth_common::settings::load_env();
    let manual = std::env::var("MANUAL").unwrap_or("".to_string()).len() > 0;
    let wait_time = std::env::var("WAIT_TIME")?.parse::<u64>()?;
    let nanokvm = NanoKVM {
        server: std::env::var("NANOKVM_SERVER")?,
        username: std::env::var("NANOKVM_USERNAME")?,
        password: std::env::var("NANOKVM_PASSWORD")?,
    };

    return Ok(Config {
        manual,
        wait_time,
        nanokvm,
    });
}

pub async fn load_container() -> Result<Container, Box<dyn Error>> {
    vermouth_common::settings::load_env();
    let server = std::env::var("SERVER")?;
    let token = std::env::var("TOKEN")?;
    let server_key = std::env::var("SERVER_KEY")?;
    fetch_container(&server, &token, &server_key).await
}
