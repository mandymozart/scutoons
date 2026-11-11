//! Session tokens (LIVE.md B.6): the API creates a session (`POST /api/live/sessions`, the `live` role) and hands the
//! conductor an HMAC-signed `{ session, role, exp, show? }`; the relay checks it with the secret it shares with the API.
//! Wire form: `base64url(payload JSON) "." base64url(HMAC-SHA256(secret, the first part))` – no padding, like a JWT
//! without its header (one algorithm, nothing to negotiate). Without a secret (`LIVE_TOKEN_SECRET` unset) the relay runs
//! in dev mode: any non-empty token passes, and it says so at start.
//! #478: a client's proof (`hello.proof`, the API's `GET /api/live/proof`) is the same scheme over
//! `{ session, facts: { account, approved }, exp }` – the facts a role's filter asks that only the API knows. A token is no
//! proof and a proof no token: each payload lacks the other's required field. In dev mode a proof's facts are taken as stated.
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use protocol::{Account, Role};
use serde::de::DeserializeOwned;
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

/// #478: what the API states of a client – facts, never who it is (no account id, no name, no mail).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    /// its account roles among member, creator, admin, live (none: a guest)
    #[serde(default)]
    pub account: Vec<Account>,
    /// an approved (paired) deck
    #[serde(default)]
    pub approved: bool,
}

/// #478: a proof's payload – the facts for one session until `exp` (unix seconds); the field order is the API's
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    pub session: String,
    pub facts: Facts,
    pub exp: i64,
}

pub fn sign<T: Serialize>(secret: &[u8], claims: &T) -> String {
    let payload = B64.encode(serde_json::to_vec(claims).expect("claims serialize"));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
    mac.update(payload.as_bytes());
    format!("{payload}.{}", B64.encode(mac.finalize().into_bytes()))
}

/// The payload of `token`: with a secret after its signature is checked (constant time); without one (dev mode) as stated.
fn open<T: DeserializeOwned>(secret: Option<&[u8]>, token: &str) -> Result<T, &'static str> {
    let (payload, sig) = token.split_once('.').ok_or("malformed token")?;
    if let Some(secret) = secret {
        let sig = B64.decode(sig).map_err(|_| "malformed token")?;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
        mac.update(payload.as_bytes());
        mac.verify_slice(&sig).map_err(|_| "bad signature")?;
    }
    serde_json::from_slice(&B64.decode(payload).map_err(|_| "malformed token")?).map_err(|_| "malformed token")
}

/// Check the signature (constant time), the session, the role and the expiry (`now` in unix seconds).
pub fn verify(secret: &[u8], token: &str, session: &str, role: Role, now: i64) -> Result<Claims, &'static str> {
    let claims: Claims = open(Some(secret), token)?;
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

/// #478: the facts of a proof for `session` at `now` (unix seconds) – signature, session and expiry checked. Without a
/// secret (dev mode) the signature is not checked: the facts are taken as stated, as any session token passes there.
pub fn proof(secret: Option<&[u8]>, token: &str, session: &str, now: i64) -> Result<Facts, &'static str> {
    let p: Proof = open(secret, token)?;
    if p.session != session {
        return Err("proof for another session");
    }
    if p.exp <= now {
        return Err("proof expired");
    }
    Ok(p.facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"shared-with-the-api";

    fn claims() -> Claims {
        Claims { session: "A7K2".into(), role: Role::Conductor, exp: 2_000, show: Some("show:catpope:1".into()) }
    }

    /// the same vector as server/tests/Unit/Http/ShowsApiTest.php
    #[test]
    fn signs_like_the_api() {
        let claims = Claims { session: "A7K2".into(), role: Role::Conductor, exp: 2000, show: Some("top".into()) };
        assert_eq!(sign(b"shared-with-the-api", &claims), "eyJzZXNzaW9uIjoiQTdLMiIsInJvbGUiOiJjb25kdWN0b3IiLCJleHAiOjIwMDAsInNob3ciOiJ0b3AifQ.VKZMHFY4kDbr-uvH_Wiu-Pw07Cpc9UdvLkp6oc8B-6Y");
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

    #[test]
    fn a_proof_and_a_token_are_not_each_other() {
        let facts = Facts { account: vec![Account::Member], approved: false };
        let p = sign(SECRET, &Proof { session: "A7K2".into(), facts: facts.clone(), exp: 2_000 });
        assert_eq!(proof(Some(SECRET), &p, "A7K2", 1_000), Ok(facts.clone()));
        assert_eq!(verify(SECRET, &p, "A7K2", Role::Conductor, 1_000), Err("malformed token"), "a proof is no session token");
        let t = sign(SECRET, &claims());
        assert_eq!(proof(Some(SECRET), &t, "A7K2", 1_000), Err("malformed token"), "a session token is no proof");
        // dev mode: the facts as stated, whatever signs them – the session and the expiry still count
        assert_eq!(proof(None, &sign(b"", &Proof { session: "A7K2".into(), facts: facts.clone(), exp: 2_000 }), "A7K2", 1_000), Ok(facts));
        assert_eq!(proof(None, &p, "B000", 1_000), Err("proof for another session"));
        assert_eq!(proof(None, &p, "A7K2", 2_000), Err("proof expired"));
    }
}
