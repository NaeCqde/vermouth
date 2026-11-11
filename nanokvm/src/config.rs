use dotenvy;
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
    let manual = dotenvy::var("MANUAL").unwrap_or("".to_string()).len() > 0;
    let wait_time = dotenvy::var("WAIT_TIME")?.parse::<u64>()?;
    let nanokvm = NanoKVM {
        server: dotenvy::var("NANOKVM_SERVER")?,
        username: dotenvy::var("NANOKVM_USERNAME")?,
        password: dotenvy::var("NANOKVM_PASSWORD")?,
    };

    return Ok(Config {
        manual,
        wait_time,
        nanokvm,
    });
}

pub async fn load_container() -> Result<Container, Box<dyn Error>> {
    let server = dotenvy::var("SERVER")?;
    let token = dotenvy::var("TOKEN")?;
    fetch_container(&server, &token).await
}
