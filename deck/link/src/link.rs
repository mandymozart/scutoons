//! Request → poll → token (CYBERDECKS.md §9.2–9.5; server/docs/API.md "Decks"). `Link::step` is called by the deck
//! now and then (every few seconds while a code waits, then whenever the token is due); it answers what the deck shows:
//! `Pending { code }` (the splash shows the code and the QR of `pairing_url`) or `Ready { token }` (the API is open,
//! `Authorization: Bearer <token>`, read-only with the member's rights). Revoked or expired → the next step asks again
//! and answers `Pending` with a new code – the build never becomes useless. Network trouble is an `Err`: the caller
//! keeps playing what it has (built-in scenes, the cache) and steps again later.

use crate::identity::Identity;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// The server's token lives an hour (Decks::TOKEN_SECONDS); the deck renews it well before
pub const RENEW_AFTER: Duration = Duration::from_secs(50 * 60);

/// One POST to the API: the status and the JSON body (also for 4xx – the protocol reads them)
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

/// How the link reaches the API (`HttpTransport`; tests use a fake server). `path` is below the API base, e.g.
/// `/decks/request`.
pub trait Transport {
    fn post(&self, path: &str, body: &Value) -> Result<Reply>;
}

/// What the deck says about itself in its request (shown to the member before approving)
#[derive(Debug, Clone, Default)]
pub struct DeckInfo {
    pub build: String,
    pub class: String,
    pub hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// waiting for a member: show the code (`format_code`) and the QR of `pairing_url`
    Pending { code: String },
    /// approved: the token for the API, and the name the member gave the deck
    Ready { token: String, name: String },
}

pub struct Link<T: Transport> {
    transport: T,
    identity: Identity,
    info: DeckInfo,
    token: Option<(String, String, Instant)>,
    renew_after: Duration,
}

impl<T: Transport> Link<T> {
    pub fn new(transport: T, identity: Identity, info: DeckInfo) -> Self {
        Self {
            transport,
            identity,
            info,
            token: None,
            renew_after: RENEW_AFTER,
        }
    }

    /// A shorter renewal (tests)
    pub fn with_renew_after(mut self, renew_after: Duration) -> Self {
        self.renew_after = renew_after;
        self
    }

    pub fn public_key_hex(&self) -> String {
        self.identity.public_key_hex()
    }

    /// The current token while it is fresh
    pub fn token(&self, now: Instant) -> Option<&str> {
        self.token
            .as_ref()
            .filter(|(_, _, at)| now.duration_since(*at) < self.renew_after)
            .map(|(t, _, _)| t.as_str())
    }

    /// The API answered 401 to the token (revoked, ended): the next step renews or asks again
    pub fn forget_token(&mut self) {
        self.token = None;
    }

    /// One step of the handshake → what the deck shows now
    pub fn step(&mut self, now: Instant) -> Result<State> {
        if let Some((token, name, at)) = &self.token {
            if now.duration_since(*at) < self.renew_after {
                return Ok(State::Ready {
                    token: token.clone(),
                    name: name.clone(),
                });
            }
        }
        self.token = None;
        // twice at most: an approved deck may come back from a request as approved, and then needs its challenge
        for _ in 0..2 {
            match self.challenge()? {
                Poll::Pending(code) => return Ok(State::Pending { code }),
                Poll::Challenge(challenge) => match self.exchange(&challenge)? {
                    Some((token, name)) => {
                        self.token = Some((token.clone(), name.clone(), now));
                        return Ok(State::Ready { token, name });
                    }
                    None => {
                        if let Some(code) = self.request()? {
                            return Ok(State::Pending { code });
                        }
                    }
                },
                Poll::AskAgain => {
                    if let Some(code) = self.request()? {
                        return Ok(State::Pending { code });
                    }
                }
            }
        }
        bail!("the server keeps answering approved without a challenge")
    }

    /// POST /decks/request → the code (pending), or None (its approval holds)
    fn request(&self) -> Result<Option<String>> {
        let body = json!({
            "public_key": self.identity.public_key_hex(),
            "build": self.info.build,
            "class": self.info.class,
            "hostname": self.info.hostname,
        });
        let reply = self.transport.post("/decks/request", &body)?;
        let deck = expect(&reply, &[200, 201], "request")?;
        match deck["status"].as_str() {
            Some("pending") => Ok(Some(text(&deck["code"], "code")?)),
            Some("approved") => Ok(None),
            other => bail!("unexpected request status {other:?}"),
        }
    }

    fn challenge(&self) -> Result<Poll> {
        let reply = self.transport.post(
            "/decks/challenge",
            &json!({ "public_key": self.identity.public_key_hex() }),
        )?;
        if reply.status == 401 {
            return Ok(Poll::AskAgain);
        }
        let deck = expect(&reply, &[200, 202], "challenge")?;
        match deck["status"].as_str() {
            Some("pending") => Ok(Poll::Pending(text(&deck["code"], "code")?)),
            Some("approved") => Ok(Poll::Challenge(text(&deck["challenge"], "challenge")?)),
            other => bail!("unexpected challenge status {other:?}"),
        }
    }

    /// The signed challenge → Some((token, name)); None when the approval ended meanwhile (401 revoked / expired)
    fn exchange(&self, challenge: &str) -> Result<Option<(String, String)>> {
        let body = json!({
            "public_key": self.identity.public_key_hex(),
            "challenge": challenge,
            "signature": self.identity.sign_challenge(challenge),
        });
        let reply = self.transport.post("/decks/token", &body)?;
        if reply.status == 401 {
            let code = reply.body["error"].as_str().unwrap_or("");
            if matches!(
                code,
                "revoked" | "expired" | "unknown_deck" | "not_approved" | "request_expired"
            ) {
                return Ok(None);
            }
            bail!("token refused: {code}");
        }
        if reply.status != 200 {
            bail!("token: HTTP {}", reply.status);
        }
        let name = reply.body["deck"]["name"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(Some((text(&reply.body["token"], "token")?, name)))
    }
}

enum Poll {
    Pending(String),
    Challenge(String),
    AskAgain,
}

fn expect<'a>(reply: &'a Reply, ok: &[u16], what: &str) -> Result<&'a Value> {
    if !ok.contains(&reply.status) {
        let code = reply.body["error"].as_str().unwrap_or("");
        bail!("{what}: HTTP {} {code}", reply.status);
    }
    Ok(&reply.body["deck"])
}

fn text(value: &Value, what: &str) -> Result<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("the answer has no {what}"))
}

/// The code as the splash shows it: K7Q42M → K7Q-42M
pub fn format_code(code: &str) -> String {
    if code.len() == 6 && code.is_ascii() {
        format!("{}-{}", &code[..3], &code[3..])
    } else {
        code.to_string()
    }
}

/// What the splash's QR code opens: the pairing page of the viewer's account page with the code prefilled
pub fn pairing_url(app_url: &str, code: &str) -> String {
    format!(
        "{}/account/decks?deck={code}",
        app_url.trim_end_matches('/')
    )
}
