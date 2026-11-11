mod config;

use config::CONFIG;
use ntex::{
    http::Response,
    web::{self, HttpServer},
};
use std::{env, error::Error, process::exit};
use std::io::Read;
use vermouth_common::{
    device_public_b64, new_server_keypair, new_token, server_public_b64, ServerAnswer, MAX_REQUEST_LEN, MIN_TOKEN_LEN,
};

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    if let Some(code) = tools() {
        exit(code);
    }

    let (host, port) = match listen_address() {
        Ok(a) => a,
        Err(e) => {
            log::error!("{}", e);
            log::error!("vermouth-server [HOST PORT]   (without arguments: LISTEN=host:port, or \"listen\" in config.json)");
            exit(1);
        }
    };

    let public = match server_public_b64(&CONFIG.server_key) {
        Ok(p) => p,
        Err(e) => {
            log::error!("server_key in config.json: {} (make one with: vermouth-server server-key)", e);
            exit(1);
        }
    };
    log::info!("SERVER_KEY for the clients: {}", public);
    log::info!("{} device(s)", CONFIG.devices.len());

    log::info!("listening on {}:{}", host, port);
    Ok(HttpServer::new(|| web::App::new().route("/", web::post().to(fetch_password)))
        .workers(1)
        .bind((host.as_str(), port))?
        .run()
        .await?)
}

/// The address to listen on: the two arguments `HOST PORT` (as before), else the environment variable `LISTEN`
/// (`host:port`), else `"listen"` in config.json.
fn listen_address() -> Result<(String, u16), String> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        3 => {
            let port = args[2].parse::<u16>().map_err(|e| format!("port: {}", e))?;
            Ok((args[1].clone(), port))
        }
        1 => {
            let listen = env::var("LISTEN")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| CONFIG.listen.clone())
                .ok_or("no address to listen on")?;
            parse_listen(&listen)
        }
        _ => Err("wrong arguments".to_string()),
    }
}

fn parse_listen(s: &str) -> Result<(String, u16), String> {
    let (host, port) = s.rsplit_once(':').ok_or_else(|| format!("listen \"{}\" is not host:port", s))?;
    let port = port.parse::<u16>().map_err(|e| format!("listen \"{}\": port: {}", s, e))?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(format!("listen \"{}\" has no host", s));
    }
    Ok((host.to_string(), port))
}

/// The helper commands (they do not need config.json): `token`, `key`, `server-key`. Returns the exit code when one ran.
fn tools() -> Option<i32> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return None;
    }
    match args[1].as_str() {
        "token" => match new_token() {
            Ok(token) => {
                println!("TOKEN={}   (for the .env of the device; keep it secret)", token);
                println!("\"key\": \"{}\"   (for \"devices\" in config.json)", device_public_b64(&token));
                Some(0)
            }
            Err(e) => {
                eprintln!("{}", e);
                Some(1)
            }
        },
        "key" => {
            let mut token = String::new();
            if std::io::stdin().read_to_string(&mut token).is_err() || token.trim().len() < MIN_TOKEN_LEN {
                eprintln!("give the token on the standard input (at least {} characters)", MIN_TOKEN_LEN);
                return Some(1);
            }
            println!("{}", device_public_b64(&token));
            Some(0)
        }
        "server-key" => match new_server_keypair() {
            Ok((secret, public)) => {
                println!("\"server_key\": \"{}\"   (secret: for config.json)", secret);
                println!("SERVER_KEY={}   (public: for the .env of the devices)", public);
                Some(0)
            }
            Err(e) => {
                eprintln!("{}", e);
                Some(1)
            }
        },
        _ => None,
    }
}

async fn fetch_password(body: ntex::util::Bytes) -> Response {
    if body.len() > MAX_REQUEST_LEN {
        return Response::NotFound().finish();
    }
    // everything that is not a valid first message for this server's key gets the same answer: 404
    let Ok(request) = ServerAnswer::open(&CONFIG.server_key, &body) else {
        return Response::NotFound().finish();
    };
    let device = request.device_b64();
    let Some((name, found)) = CONFIG.devices.iter().find(|(_, d)| d.key == device) else {
        log::warn!("unknown device key {}", device);
        return Response::NotFound().finish();
    };
    log::info!("device {}", name);

    let sealed = serde_json::to_vec(&found.container)
        .map_err(|e| e.to_string())
        .and_then(|json| request.answer(&json).map_err(|e| e.to_string()));
    match sealed {
        Ok(body) => Response::Ok().content_type("application/octet-stream").body(body),
        Err(e) => {
            log::error!("{}", e);
            Response::InternalServerError().finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_listen;

    #[test]
    fn listen_forms() {
        assert_eq!(parse_listen("0.0.0.0:9810"), Ok(("0.0.0.0".to_string(), 9810)));
        assert_eq!(parse_listen("[::]:9810"), Ok(("::".to_string(), 9810)));
        assert_eq!(parse_listen("localhost:80"), Ok(("localhost".to_string(), 80)));
        assert!(parse_listen("9810").is_err());
        assert!(parse_listen("host:notaport").is_err());
        assert!(parse_listen(":80").is_err());
    }
}
