mod config;

#[global_allocator]
static ALLOC: vermouth_common::ZeroOnFree = vermouth_common::ZeroOnFree;

use std::{error::Error, process::exit};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    match config::load().await {
        Ok(config) => {
            let c = &config.container;
            let mut cmd = if cfg!(windows) {
                let mut cmd = Command::new("cmd");
                cmd.args(["/c", "run.bat"]);
                cmd
            } else {
                let mut cmd = Command::new("sh");
                cmd.arg("run.sh");
                cmd
            };
            cmd.env("SOURCE", &c.source).env("MOUNT", &c.mount).env("PASSWORD", &c.password);
            let _ = cmd.spawn().expect("run failure").wait().await;
            // `cmd` (with its copy of the password) and `config` (wiped on drop) go out of scope here
            drop(cmd);

            return Ok(());
        }
        Err(e) => {
            log::error!("{}", e);
            exit(1);
        }
    };
}
