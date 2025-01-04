use dotenvy;
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub path: String,
    pub mount: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub token: String,
    pub container: Container,
}

pub async fn load() -> Result<Config, Box<dyn Error>> {
    let server = dotenvy::var("SERVER")?;
    let token = dotenvy::var("TOKEN")?;

    let client = Client::default();

    let resp = client
        .post(server)
        .header(header::AUTHORIZATION, &token)
        .send()
        .await?;

    let status = resp.status();
    if status != StatusCode::OK {
        return Err(format!("request failure: {}", status.as_str()).into());
    }

    let container = resp.json::<Container>().await?;
    return Ok(Config { token, container });
}
