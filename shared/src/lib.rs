//! What the server, the client and nanokvm have in common: the secure channel.
//!
//! The channel is a Noise `IK` handshake (`Noise_IK_25519_ChaChaPoly_BLAKE2s`) carried in one HTTP request and its answer:
//!
//! * Every device has its own `TOKEN`. The device's key pair is derived from it (HKDF-SHA256 -> X25519), so the server
//!   never needs the token: it stores only the **public key** of the device (a one-way image of the token).
//! * The client knows (pins) the static public key of the server (`SERVER_KEY`). Without the server's private key nobody can
//!   answer, so a router that spoofs the server is rejected.
//! * Both sides are authenticated by their static keys; the session keys come from ephemeral keys on both sides (forward
//!   secrecy: a later leak of a token or of the server key does not open a recorded answer).
//! * The device's public key is sent encrypted. The source IP is not used for anything.
//! * The answer can only be opened by the one who sent the request, so a recorded request replayed by the router gets an
//!   answer that nobody can read.
//!
//! Use long random tokens: `vermouth-server keygen` makes one.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hkdf::Hkdf;
use sha2::Sha256;
use std::error::Error;
use x25519_dalek::{PublicKey, StaticSecret};

#[cfg(feature = "client")]
mod fetch;
#[cfg(feature = "client")]
pub use fetch::{fetch_container, Container};

pub const NOISE_PARAMS: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
/// A token shorter than this is too easy to guess offline from the public key.
pub const MIN_TOKEN_LEN: usize = 32;
/// The first message of `IK` with an empty payload is 96 bytes; anything much bigger is not ours.
pub const MAX_REQUEST_LEN: usize = 256;

pub type Key = [u8; 32];

fn key_from_b64(s: &str) -> Result<Key, Box<dyn Error>> {
    let v = B64.decode(s.trim())?;
    v.try_into().map_err(|_| "a key must be 32 bytes (base64)".into())
}

/// A new random token (base64 of 32 random bytes).
pub fn new_token() -> Result<String, Box<dyn Error>> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).map_err(|e| format!("no random numbers: {e}"))?;
    Ok(B64.encode(b))
}

/// The private key of a device, from its token.
pub fn device_secret(token: &str) -> Key {
    let hk = Hkdf::<Sha256>::new(Some(b"vermouth v2"), token.trim().as_bytes());
    let mut k = [0u8; 32];
    hk.expand(b"device key", &mut k).expect("32 bytes");
    k
}

pub fn public_of(secret: &Key) -> Key {
    PublicKey::from(&StaticSecret::from(*secret)).to_bytes()
}

/// The public key of the device with this token: this is what the server's config.json holds instead of the token.
pub fn device_public_b64(token: &str) -> String {
    B64.encode(public_of(&device_secret(token)))
}

/// A new key pair for the server: (private, public), both base64.
pub fn new_server_keypair() -> Result<(String, String), Box<dyn Error>> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).map_err(|e| format!("no random numbers: {e}"))?;
    Ok((B64.encode(b), B64.encode(public_of(&b))))
}

pub fn server_public_b64(secret_b64: &str) -> Result<String, Box<dyn Error>> {
    Ok(B64.encode(public_of(&key_from_b64(secret_b64)?)))
}

fn params() -> snow::params::NoiseParams {
    NOISE_PARAMS.parse().expect("valid noise parameters")
}

/// The client side of one request.
pub struct ClientHandshake {
    hs: snow::HandshakeState,
}

impl ClientHandshake {
    /// Returns the handshake and the first message (the body of the request).
    pub fn start(token: &str, server_public_b64: &str) -> Result<(ClientHandshake, Vec<u8>), Box<dyn Error>> {
        if token.trim().len() < MIN_TOKEN_LEN {
            return Err(format!("TOKEN must be at least {MIN_TOKEN_LEN} characters (make one with: vermouth-server token)").into());
        }
        let secret = device_secret(token);
        let server_public = key_from_b64(server_public_b64)?;
        let mut hs = snow::Builder::new(params())
            .local_private_key(&secret)
            .remote_public_key(&server_public)
            .build_initiator()?;
        let mut msg = vec![0u8; MAX_REQUEST_LEN];
        let n = hs.write_message(&[], &mut msg)?;
        msg.truncate(n);
        Ok((ClientHandshake { hs }, msg))
    }

    /// Opens the answer. Fails when the sender is not the server with the pinned key, or the data was changed.
    pub fn finish(mut self, answer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut out = vec![0u8; answer.len()];
        let n = self
            .hs
            .read_message(answer, &mut out)
            .map_err(|_| "the answer cannot be opened (wrong SERVER_KEY or TOKEN, or it was changed on the way)")?;
        out.truncate(n);
        Ok(out)
    }
}

/// The server side of one request.
pub struct ServerAnswer {
    /// The public key of the device that asked (known only after its message was opened).
    pub device: Key,
    hs: snow::HandshakeState,
}

impl ServerAnswer {
    /// Opens the first message of a client. Fails for anything that was not made for this server.
    pub fn open(server_secret_b64: &str, request: &[u8]) -> Result<ServerAnswer, Box<dyn Error>> {
        if request.len() > MAX_REQUEST_LEN {
            return Err("request too long".into());
        }
        let secret = key_from_b64(server_secret_b64)?;
        let mut hs = snow::Builder::new(params()).local_private_key(&secret).build_responder()?;
        let mut buf = vec![0u8; MAX_REQUEST_LEN];
        hs.read_message(request, &mut buf)?;
        let device: Key = hs.get_remote_static().ok_or("no device key")?.try_into().map_err(|_| "bad device key")?;
        Ok(ServerAnswer { device, hs })
    }

    pub fn device_b64(&self) -> String {
        B64.encode(self.device)
    }

    /// The answer (the second message) carrying `payload`.
    pub fn answer(mut self, payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut out = vec![0u8; payload.len() + 128];
        let n = self.hs.write_message(payload, &mut out)?;
        out.truncate(n);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> (String, String) {
        new_server_keypair().unwrap()
    }

    #[test]
    fn round_trip_and_server_learns_the_device() {
        let (sk, sp) = server();
        let token = new_token().unwrap();
        let (c, req) = ClientHandshake::start(&token, &sp).unwrap();
        let a = ServerAnswer::open(&sk, &req).unwrap();
        assert_eq!(a.device_b64(), device_public_b64(&token));
        let ans = a.answer(b"{\"password\":\"secret\"}").unwrap();
        assert!(!String::from_utf8_lossy(&ans).contains("secret"));
        assert_eq!(c.finish(&ans).unwrap(), b"{\"password\":\"secret\"}");
    }

    #[test]
    fn short_token_is_refused() {
        assert!(ClientHandshake::start("", &server().1).is_err());
        assert!(ClientHandshake::start("short", &server().1).is_err());
    }

    #[test]
    fn requests_differ_every_time_and_hide_the_device() {
        let (_, sp) = server();
        let token = new_token().unwrap();
        let (_, r1) = ClientHandshake::start(&token, &sp).unwrap();
        let (_, r2) = ClientHandshake::start(&token, &sp).unwrap();
        assert_ne!(r1, r2);
        let pubkey = B64.decode(device_public_b64(&token)).unwrap();
        assert!(!r1.windows(32).any(|w| w == pubkey.as_slice()));
    }

    #[test]
    fn a_fake_server_cannot_answer() {
        let (_, pinned) = server();
        let (fake_sk, _) = server();
        let token = new_token().unwrap();
        let (c, req) = ClientHandshake::start(&token, &pinned).unwrap();
        // the fake does not have the private key of the pinned key: it cannot even open the request
        assert!(ServerAnswer::open(&fake_sk, &req).is_err());
        // and an answer made by a handshake of its own does not open for the client
        let (_, other) = ClientHandshake::start(&token, &server().1).unwrap();
        let fake = ServerAnswer::open(&fake_sk, &ClientHandshake::start(&token, &server_public_b64(&fake_sk).unwrap()).unwrap().1)
            .unwrap()
            .answer(b"x")
            .unwrap();
        assert!(c.finish(&fake).is_err());
        let _ = other;
    }

    #[test]
    fn replayed_request_gets_an_answer_nobody_else_can_open() {
        let (sk, sp) = server();
        let token = new_token().unwrap();
        let (c, req) = ClientHandshake::start(&token, &sp).unwrap();
        let a1 = ServerAnswer::open(&sk, &req).unwrap().answer(b"first").unwrap();
        // the router replays the recorded request: the server answers, but with fresh keys the router cannot get to
        let a2 = ServerAnswer::open(&sk, &req).unwrap().answer(b"first").unwrap();
        assert_ne!(a1, a2);
        // the answer to a replay belongs to the original request only: a handshake of another request does not open it
        let (c2, _) = ClientHandshake::start(&token, &sp).unwrap();
        assert!(c2.finish(&a2).is_err());
        assert!(c.finish(&a1).is_ok());
    }

    #[test]
    fn tampering_and_garbage_are_refused() {
        let (sk, sp) = server();
        let token = new_token().unwrap();
        let (c, mut req) = ClientHandshake::start(&token, &sp).unwrap();
        let mut ans = ServerAnswer::open(&sk, &req).unwrap().answer(b"x").unwrap();
        *ans.last_mut().unwrap() ^= 1;
        assert!(c.finish(&ans).is_err());
        *req.last_mut().unwrap() ^= 1;
        assert!(ServerAnswer::open(&sk, &req).is_err());
        assert!(ServerAnswer::open(&sk, b"hello").is_err());
        assert!(ServerAnswer::open(&sk, &[0u8; 1000]).is_err());
    }
}
