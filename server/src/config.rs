use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, error::Error, fs::File, io::BufReader, net::IpAddr, process::exit,
};

pub static CONFIG: Lazy<Config> = Lazy::new(|| match load() {
    Ok(c) => c,
    Err(e) => {
        log::error!("{}", e);
        exit(1);
    }
});

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub source: String,
    pub mount: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Device {
    /// The public key of the device (base64), made from its token with `vermouth-server key`. The token itself is not stored.
    pub key: String,
    /// The source IP address the device must connect from (both the key and the address have to match).
    pub ip: IpAddr,
    #[serde(flatten)]
    pub container: Container,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// The private key of the server (base64), made with `vermouth-server server-key`. Keep config.json private.
    pub server_key: String,
    /// `host:port` to listen on when the program is started without arguments (the environment variable `LISTEN` wins).
    #[serde(default)]
    pub listen: Option<String>,
    /// The devices by name (any name, for the log).
    pub devices: HashMap<String, Device>,
}

pub fn load() -> Result<Config, Box<dyn Error>> {
    let file = File::open("config.json")?;
    let reader = BufReader::new(file);

    let config: Config = serde_json::from_reader(reader)?;
    return Ok(config);
}
