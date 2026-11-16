use crate::ClientHandshake;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::error::Error;
use zeroize::{Zeroize, Zeroizing};

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub source: String,
    pub mount: String,
    pub password: String,
}

/// The password is wiped when the container is dropped.
impl Drop for Container {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

/// Asks the server for the container of this device. `token` is the device's own token (never sent), `server_key` the pinned
/// public key of the server (see the crate documentation).
pub async fn fetch_container(server: &str, token: &str, server_key: &str) -> Result<Container, Box<dyn Error>> {
    let (hs, request) = ClientHandshake::start(token, server_key)?;

    let resp = Client::default()
        .post(server)
        .header("content-type", "application/octet-stream")
        .body(request)
        .send()
        .await?;

    let status = resp.status();
    if status != StatusCode::OK {
        return Err(format!("request failure: {}", status.as_str()).into());
    }

    // the decrypted answer holds the password: wipe it as soon as it is parsed
    let plain = Zeroizing::new(hs.finish(&resp.bytes().await?)?);
    Ok(serde_json::from_slice(&plain)?)
}
