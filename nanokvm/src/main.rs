mod config;

// wipes every freed heap block, so no copy of the password stays in this long-running process
#[global_allocator]
static ALLOC: vermouth_common::ZeroOnFree = vermouth_common::ZeroOnFree;

use reqwest::Client;
use serde::Serialize;
use std::{error::Error, process::exit, time::Duration};
use tokio::{fs, time::sleep};
use zeroize::Zeroizing;

#[derive(Serialize)]
struct Paste<'a> {
    content: &'a str,
    langue: &'a str,
}

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
        log::info!("Manual Inputting...");
        if let Err(e) = input_password(&config).await {
            log::error!("{}", e);
            exit(1);
        };
    } else {
        log::info!("Auto Input Mode");
        let mut old_power: bool = false;

        loop {
            let power: bool = match fs::read_to_string("/sys/class/gpio/gpio504/value").await {
                Ok(text) => text.starts_with("0"),
                Err(e) => {
                    log::error!("{}", e);
                    exit(1);
                }
            };
            log::debug!("Power: {}", power);

            if !old_power && power {
                log::info!("Computer is power on");
                let _ = reset_hid(&config).await;
                log::info!("Waiting {}s...", config.wait_time);
                sleep(Duration::from_secs(config.wait_time)).await;
                log::info!("Waiting complete");

                if let Err(e) = input_password(&config).await {
                    log::error!("{}", e);
                    exit(1);
                };
            }
            if old_power && !power {
                log::info!("Computer is power off");
            }

            old_power = power;
            sleep(Duration::from_secs(1)).await;
        }
    }
    Ok(())
}

async fn reset_hid(config: &config::Config) -> Result<(), Box<dyn Error>> {
    let client = Client::default();
    client
        .post(format!("{}/api/hid/reset", config.nanokvm.server))
        .send()
        .await?;
    log::info!("Reset HID");
    Ok(())
}

async fn input_password(config: &config::Config) -> Result<(), Box<dyn Error>> {
    log::info!("Loading password...");
    let container = config::load_container().await?;
    log::info!("Loaded password");
    log::info!("Pasting password...");
    let client = Client::default();

    // `container` is wiped when it is dropped; `content` is wiped explicitly
    let content = Zeroizing::new(format!("{}\n", container.password));
    let payload = Paste { content: &content, langue: "en" };
    client
        .post(format!("{}/api/hid/paste", config.nanokvm.server))
        .json(&payload)
        .send()
        .await?;
    log::info!("Pasted password");

    return Ok(());
}
