//! Session tokens (LIVE.md B.6): the API creates a session (`POST /api/live/sessions`, the `live` role) and hands the
//! conductor an HMAC-signed `{ session, role, exp, show? }`; the relay checks it with the secret it shares with the API.
//! Wire form: `base64url(payload JSON) "." base64url(HMAC-SHA256(secret, the first part))` – no padding, like a JWT
//! without its header (one algorithm, nothing to negotiate). Without a secret (`LIVE_TOKEN_SECRET` unset) the relay runs
//! in dev mode: any non-empty token passes, and it says so at start.
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use protocol::Role;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    /// the session code
    pub session: String,
    pub role: Role,
    /// expiry, unix seconds
    pub exp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
}

pub fn sign(secret: &[u8], claims: &Claims) -> String {
    let payload = B64.encode(serde_json::to_vec(claims).expect("claims serialize"));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
    mac.update(payload.as_bytes());
    format!("{payload}.{}", B64.encode(mac.finalize().into_bytes()))
}

/// Check the signature (constant time), the session, the role and the expiry (`now` in unix seconds).
pub fn verify(secret: &[u8], token: &str, session: &str, role: Role, now: i64) -> Result<Claims, &'static str> {
    let (payload, sig) = token.split_once('.').ok_or("malformed token")?;
    let sig = B64.decode(sig).map_err(|_| "malformed token")?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
    mac.update(payload.as_bytes());
    mac.verify_slice(&sig).map_err(|_| "bad signature")?;
    let claims: Claims =
        serde_json::from_slice(&B64.decode(payload).map_err(|_| "malformed token")?).map_err(|_| "malformed token")?;
    if claims.session != session {
        return Err("token for another session");
    }
    if claims.role != role {
        return Err("token for another role");
    }
    if claims.exp <= now {
        return Err("token expired");
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"shared-with-the-api";

    fn claims() -> Claims {
        Claims { session: "A7K2".into(), role: Role::Conductor, exp: 2_000, show: Some("show:catpope:1".into()) }
    }

    #[test]
    fn round_trip() {
        let t = sign(SECRET, &claims());
        assert_eq!(verify(SECRET, &t, "A7K2", Role::Conductor, 1_000), Ok(claims()));
    }

    #[test]
    fn refusals() {
        let t = sign(SECRET, &claims());
        assert_eq!(verify(b"other", &t, "A7K2", Role::Conductor, 1_000), Err("bad signature"));
        assert_eq!(verify(SECRET, &t, "B000", Role::Conductor, 1_000), Err("token for another session"));
        assert_eq!(verify(SECRET, &t, "A7K2", Role::Bridge, 1_000), Err("token for another role"));
        assert_eq!(verify(SECRET, &t, "A7K2", Role::Conductor, 2_000), Err("token expired"));
        assert_eq!(verify(SECRET, "nodot", "A7K2", Role::Conductor, 1_000), Err("malformed token"));
        // a payload edited after signing
        let (_, sig) = t.split_once('.').unwrap();
        let forged = format!("{}.{sig}", B64.encode(br#"{"session":"A7K2","role":"conductor","exp":9999999999}"#));
        assert_eq!(verify(SECRET, &forged, "A7K2", Role::Conductor, 1_000), Err("bad signature"));
    }
}
