use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    uid: String,
    exp: i64,
}

/// Create a compact, HMAC-signed session token: `base64(payload).base64(sig)`.
pub fn sign_session(secret: &str, user_id: &str, now: i64, ttl_secs: i64) -> String {
    let claims = Claims {
        uid: user_id.to_string(),
        exp: now + ttl_secs,
    };
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).expect("serialize"));
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(payload.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{payload}.{sig}")
}

/// Verify signature + expiry. Returns the user id on success.
pub fn verify_session(secret: &str, token: &str, now: i64) -> Option<String> {
    let (payload, sig) = token.split_once('.')?;
    let sig = URL_SAFE_NO_PAD.decode(sig).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&sig).ok()?;
    let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    if claims.exp < now {
        return None;
    }
    Some(claims.uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = sign_session("secret", "usr_1", 1_000, 60);
        assert_eq!(verify_session("secret", &t, 1_030).as_deref(), Some("usr_1"));
        assert!(verify_session("secret", &t, 1_061).is_none());
        assert!(verify_session("other", &t, 1_030).is_none());
        assert!(verify_session("secret", "garbage", 1_030).is_none());
    }
}
