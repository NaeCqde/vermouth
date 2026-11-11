mod config;

use config::CONFIG;
use ntex::{
    http::{header, Response},
    web::{self, HttpRequest, HttpServer},
};
use once_cell::sync::Lazy;
use std::{env, error::Error, process::exit};
use vermouth_common::{Keys, MIN_TOKEN_LEN, NONCE_HEADER};

static KEYS: Lazy<Keys> = Lazy::new(|| Keys::from_token(&CONFIG.token));

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let (host, port) = match listen_address() {
        Ok(a) => a,
        Err(e) => {
            log::error!("{}", e);
            log::error!("vermouth-server [HOST PORT]   (without arguments: LISTEN=host:port, or \"listen\" in config.json)");
            exit(1);
        }
    };

    if CONFIG.token.is_empty() {
        log::error!("the token in config.json is empty: anyone could compute the keys and read the passwords. Use a long random one (openssl rand -base64 32)");
        exit(1);
    }
    if CONFIG.token.len() < MIN_TOKEN_LEN {
        log::warn!(
            "the token is shorter than {} characters: the answers can be guessed offline from a recording. Use a long random one (openssl rand -base64 32)",
            MIN_TOKEN_LEN
        );
    }
    // the keys are made now, not at the first request
    Lazy::force(&KEYS);

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

async fn fetch_password(req: HttpRequest) -> Response {
    let header_value = |name: &str| req.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
    let (nonce, tag) = match (header_value(NONCE_HEADER), header_value(header::AUTHORIZATION.as_str())) {
        (Some(n), Some(t)) => (n, t),
        _ => return Response::NotFound().finish(),
    };
    if !KEYS.verify_request(&nonce, &tag) {
        return Response::NotFound().finish();
    }

    let ip = match req.peer_addr() {
        Some(a) => a.ip(),
        None => return Response::InternalServerError().finish(),
    };
    println!("{}", ip);

    let Some(container) = CONFIG.containers.get(&ip) else {
        return Response::NotFound().finish();
    };
    let sealed = serde_json::to_vec(container)
        .map_err(|e| e.to_string())
        .and_then(|json| KEYS.seal(&nonce, &json).map_err(|e| e.to_string()));
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
