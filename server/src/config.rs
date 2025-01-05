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
pub struct Config {
    pub token: String,
    pub containers: HashMap<IpAddr, Container>,
}

pub fn load() -> Result<Config, Box<dyn Error>> {
    let file = File::open("config.json")?;
    let reader = BufReader::new(file);

    let config: Config = serde_json::from_reader(reader)?;
    return Ok(config);
}
