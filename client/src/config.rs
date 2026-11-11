use std::error::Error;
use vermouth_common::{fetch_container, Container};

pub struct Config {
    pub container: Container,
}

pub async fn load() -> Result<Config, Box<dyn Error>> {
    let server = dotenvy::var("SERVER")?;
    let token = dotenvy::var("TOKEN")?;
    let server_key = dotenvy::var("SERVER_KEY")?;

    let container = fetch_container(&server, &token, &server_key).await?;
    Ok(Config { container })
}
