use std::error::Error;
use vermouth_common::{fetch_container, Container};

pub struct Config {
    pub container: Container,
}

pub async fn load() -> Result<Config, Box<dyn Error>> {
    vermouth_common::settings::load_env();
    let server = std::env::var("SERVER")?;
    let token = std::env::var("TOKEN")?;
    let server_key = std::env::var("SERVER_KEY")?;

    let container = fetch_container(&server, &token, &server_key).await?;
    Ok(Config { container })
}
