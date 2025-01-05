mod config;

use std::{collections::HashMap, error::Error, process::exit};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    match config::load().await {
        Ok(config) => {
            let mut map = HashMap::new();
            map.insert("SOURCE", config.container.source);
            map.insert("MOUNT", config.container.mount);
            map.insert("PASSWORD", config.container.password);

            if cfg!(windows) {
                let _ = Command::new("cmd")
                    .args(&["/c", "run.bat"])
                    .envs(&map)
                    .spawn()
                    .expect("run failure")
                    .wait()
                    .await;
            } else {
                let _ = Command::new("sh")
                    .arg("run.sh")
                    .envs(&map)
                    .spawn()
                    .expect("run failure")
                    .wait()
                    .await;
            }

            return Ok(());
        }
        Err(e) => {
            log::error!("{}", e);
            exit(1);
        }
    };
}
