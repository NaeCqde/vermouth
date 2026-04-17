mod config;

use reqwest::Client;
use std::{collections::HashMap, error::Error, process::exit, time::Duration};
use tokio::{fs, time::sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let config = match config::load().await {
        Ok(config) => config,
        Err(e) => {
            log::error!("{}", e);
            exit(1);
        }
    };

    if config.manual {
        if let Err(e) = input_password(&config).await {
            log::error!("{}", e);
            exit(1);
        };
    } else {
        let mut old_power: bool = false;

        loop {
            let power: bool = match fs::read_to_string("/sys/class/gpio/gpio504/value").await {
                Ok(text) => text == "0",
                Err(e) => {
                    log::error!("{}", e);
                    exit(1);
                }
            };

            if !old_power && power {
                sleep(Duration::from_secs(40)).await;

                if let Err(e) = input_password(&config).await {
                    log::error!("{}", e);
                    exit(1);
                };
            }

            old_power = power;
            sleep(Duration::from_secs(1)).await;
        }
    }
    Ok(())
}

async fn input_password(config: &config::Config) -> Result<(), Box<dyn Error>> {
    let container = config::load_container().await?;
    let client = Client::default();

    let mut payload = HashMap::new();
    payload.insert("content", format!("{}\n", container.password));
    payload.insert("langue", "en".to_string());
    client
        .post(format!("{}/api/hid/paste", config.nanokvm.server))
        .json(&payload)
        .send()
        .await?;

    return Ok(());
}
