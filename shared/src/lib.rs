//! What the server, the client and nanokvm have in common: the sealing of the answer.
//!
//! The answer of the server (the password of a container) is encrypted and authenticated with keys derived from the
//! shared `TOKEN`, so the HTTP between them can be read on the way without giving the password away:
//!
//! * `k_auth` and `k_enc` are made from the token with HKDF-SHA256 (two different labels).
//! * The request carries a fresh random `nonce` (header `x-vermouth-nonce`, base64) and, in the `Authorization` header,
//!   `HMAC-SHA256(k_auth, "vermouth v1 request" || nonce)` (base64). The token itself is never sent.
//! * The answer is `XChaCha20-Poly1305(k_enc)` over the JSON, with a random 24-byte nonce in front, and the request nonce as
//!   the associated data: an answer to another request (a replay) does not open.
//!
//! The token is the only secret: use a long random one (`openssl rand -base64 32`).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::error::Error;

#[cfg(feature = "client")]
mod fetch;
#[cfg(feature = "client")]
pub use fetch::{fetch_container, Container};

/// Header with the nonce of the request.
pub const NONCE_HEADER: &str = "x-vermouth-nonce";
/// A token shorter than this is too easy to guess offline from a recorded answer.
pub const MIN_TOKEN_LEN: usize = 24;

const NONCE_LEN: usize = 16;
const XNONCE_LEN: usize = 24;

type HmacSha256 = Hmac<Sha256>;

pub struct Keys {
    auth: [u8; 32],
    enc: [u8; 32],
}

impl Keys {
    pub fn from_token(token: &str) -> Keys {
        let hk = Hkdf::<Sha256>::new(Some(b"vermouth v1"), token.as_bytes());
        let mut auth = [0u8; 32];
        let mut enc = [0u8; 32];
        hk.expand(b"auth", &mut auth).expect("32 bytes");
        hk.expand(b"enc", &mut enc).expect("32 bytes");
        Keys { auth, enc }
    }

    /// A fresh random nonce for a request (base64).
    pub fn new_nonce() -> Result<String, Box<dyn Error>> {
        let mut n = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut n).map_err(|e| format!("no random numbers: {e}"))?;
        Ok(B64.encode(n))
    }

    fn mac(&self, nonce: &str) -> HmacSha256 {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.auth).expect("any key length");
        mac.update(b"vermouth v1 request");
        mac.update(nonce.as_bytes());
        mac
    }

    /// The value of the `Authorization` header of a request with this nonce.
    pub fn request_tag(&self, nonce: &str) -> String {
        B64.encode(self.mac(nonce).finalize().into_bytes())
    }

    /// Does the tag belong to this nonce and this token? (constant time)
    pub fn verify_request(&self, nonce: &str, tag: &str) -> bool {
        if nonce.len() < 16 || nonce.len() > 64 {
            return false;
        }
        match B64.decode(tag) {
            Ok(tag) => self.mac(nonce).verify_slice(&tag).is_ok(),
            Err(_) => false,
        }
    }

    /// Encrypts the answer to the request with this nonce.
    pub fn seal(&self, nonce: &str, plaintext: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut xn = [0u8; XNONCE_LEN];
        getrandom::getrandom(&mut xn).map_err(|e| format!("no random numbers: {e}"))?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.enc));
        let ct = cipher
            .encrypt(
                XNonce::from_slice(&xn),
                Payload { msg: plaintext, aad: nonce.as_bytes() },
            )
            .map_err(|_| "encryption failure")?;
        let mut out = Vec::with_capacity(XNONCE_LEN + ct.len());
        out.extend_from_slice(&xn);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypts the answer to the request with this nonce. Fails when the token is wrong, the data was changed, or it is the
    /// answer to another request.
    pub fn open(&self, nonce: &str, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        if data.len() < XNONCE_LEN + 16 {
            return Err("the answer is too short".into());
        }
        let (xn, ct) = data.split_at(XNONCE_LEN);
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.enc));
        cipher
            .decrypt(XNonce::from_slice(xn), Payload { msg: ct, aad: nonce.as_bytes() })
            .map_err(|_| "the answer cannot be decrypted (wrong TOKEN, or it was changed on the way)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn request_tag_round_trip() {
        let k = Keys::from_token(TOKEN);
        let n = Keys::new_nonce().unwrap();
        let tag = k.request_tag(&n);
        assert!(k.verify_request(&n, &tag));
        assert!(!k.verify_request(&Keys::new_nonce().unwrap(), &tag), "another nonce");
        assert!(!Keys::from_token("another token another token!").verify_request(&n, &tag));
        assert!(!k.verify_request(&n, "not base64!"));
        assert!(!k.verify_request("short", &tag));
    }

    #[test]
    fn seal_open_round_trip() {
        let k = Keys::from_token(TOKEN);
        let n = Keys::new_nonce().unwrap();
        let sealed = k.seal(&n, b"{\"password\":\"secret\"}").unwrap();
        assert!(!String::from_utf8_lossy(&sealed).contains("secret"));
        assert_eq!(k.open(&n, &sealed).unwrap(), b"{\"password\":\"secret\"}");
    }

    #[test]
    fn open_refuses_other_request_other_token_and_changes() {
        let k = Keys::from_token(TOKEN);
        let n = Keys::new_nonce().unwrap();
        let sealed = k.seal(&n, b"x").unwrap();
        assert!(k.open(&Keys::new_nonce().unwrap(), &sealed).is_err(), "replay for another request");
        assert!(Keys::from_token("another token another token!").open(&n, &sealed).is_err());
        let mut changed = sealed.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert!(k.open(&n, &changed).is_err());
        assert!(k.open(&n, &sealed[..10]).is_err());
    }

    #[test]
    fn two_seals_differ() {
        let k = Keys::from_token(TOKEN);
        let n = Keys::new_nonce().unwrap();
        assert_ne!(k.seal(&n, b"x").unwrap(), k.seal(&n, b"x").unwrap());
    }
}
