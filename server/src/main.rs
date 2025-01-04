mod config;

use config::CONFIG;
use ntex::{
    http::{header, Response},
    web::{self, guard, HttpRequest, HttpServer},
};
use std::{env, error::Error};

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let args: Vec<String> = env::args().collect();
    if 3 == args.len() {
        let host = args[1].as_str();

        match args[2].parse::<u16>() {
            Ok(port) => {
                return Ok(HttpServer::new(|| {
                    web::App::new().route(
                        "/",
                        web::post()
                            .guard(guard::Header(header::AUTHORIZATION.as_str(), &CONFIG.token))
                            .to(fetch_password),
                    )
                })
                .bind((host, port))?
                .run()
                .await?)
            }
            Err(e) => log::error!("{}", e),
        }
    }

    log::error!("vermouth-server HOST PORT");
    Ok(())
}

async fn fetch_password(req: HttpRequest) -> Response {
    let ip = req.peer_addr().expect("get client ip failure").ip();

    match CONFIG.containers.get(&ip) {
        Some(container) => Response::Ok().json(&container),
        None => Response::NotFound().body("{}"),
    }
}
