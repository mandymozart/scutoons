//! The browser deck's pairing (#470): deck-link's `Session` – the native daemon's handshake and schedule – driven by the
//! page (deck-key.ts): it fetches what an ask names and signs with a non-extractable WebCrypto Ed25519 key, so the private
//! key never exists as bytes in wasm or JS. The token lives here only (never stored); the page reads it for the one
//! request that needs it (a show medium's signed link from the API).
use deck_link::{Ask, DeckInfo, Session};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct DeckLink {
    session: Session,
}

/// an ask as the page reads it: `{ post: <path below the API>, body }` or `{ sign: <message> }`
fn ask_json(ask: Option<Ask>) -> Option<String> {
    let v = match ask? {
        Ask::Post { path, body } => serde_json::json!({ "post": path, "body": body }),
        Ask::Sign { message } => serde_json::json!({ "sign": message }),
    };
    Some(v.to_string())
}

#[wasm_bindgen]
impl DeckLink {
    /// A link for the deck's public key (32 bytes, lower-case hex – WebCrypto's raw export); `build` = the page's build label
    #[wasm_bindgen(constructor)]
    pub fn new(public_key_hex: &str, build: &str) -> Result<DeckLink, JsValue> {
        if public_key_hex.len() != 64 || !public_key_hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(JsValue::from_str("a deck key is 32 bytes as lower-case hex"));
        }
        let info = DeckInfo { build: build.chars().take(64).collect(), class: "web".into(), hostname: "browser".into() };
        Ok(DeckLink { session: Session::new(public_key_hex.to_owned(), info) })
    }

    /// The first ask of a round when one is due at `now` (ms), else undefined
    pub fn next(&mut self, now: f64) -> Option<String> {
        ask_json(self.session.next(now))
    }

    /// The answer to a `post` (its status and body text) → the next ask, undefined = the round is over
    pub fn replied(&mut self, status: u16, body: &str, now: f64) -> Option<String> {
        let body = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        ask_json(self.session.replied(status, &body, now))
    }

    /// The signature a `sign` wanted (64 bytes, hex) → the token request
    pub fn signed(&mut self, signature_hex: &str, now: f64) -> Option<String> {
        ask_json(self.session.signed(signature_hex, now))
    }

    /// The round broke off (offline, no WebCrypto)
    pub fn failed(&mut self, why: &str, now: f64) {
        self.session.failed(why, now);
    }

    /// The API refused the token (401): gone at once, asked again
    pub fn forget(&mut self, now: f64) {
        self.session.forget(now);
    }

    /// The token while it is fresh (for `Authorization: Bearer` to the API only)
    pub fn token(&self, now: f64) -> Option<String> {
        self.session.token(now).map(str::to_owned)
    }

    /// The code to pair with while a member is awaited ("" = none – the pairing card hides)
    pub fn code(&self) -> String {
        self.session.code().unwrap_or_default().to_owned()
    }

    /// Why there is no token ("" = there is one)
    pub fn why(&self, now: f64) -> String {
        self.session.why(now)
    }

    /// The deck's line about its link
    pub fn line(&self, now: f64) -> String {
        self.session.line(now)
    }
}
