use crate::{Keys, NONCE_HEADER};
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub source: String,
    pub mount: String,
    pub password: String,
}

/// Asks the server for the container of this computer. The token is not sent: the request is signed with it and the answer
/// comes encrypted with it (see the crate documentation).
pub async fn fetch_container(server: &str, token: &str) -> Result<Container, Box<dyn Error>> {
    let keys = Keys::from_token(token);
    let nonce = Keys::new_nonce()?;

    let resp = Client::default()
        .post(server)
        .header(NONCE_HEADER, &nonce)
        .header(header::AUTHORIZATION, keys.request_tag(&nonce))
        .send()
        .await?;

    let status = resp.status();
    if status != StatusCode::OK {
        return Err(format!("request failure: {}", status.as_str()).into());
    }

    let sealed = resp.bytes().await?;
    let plain = keys.open(&nonce, &sealed)?;
    Ok(serde_json::from_slice(&plain)?)
}
