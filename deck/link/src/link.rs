//! Request → poll → token (CYBERDECKS.md §9.2–9.5; server/docs/API.md "Decks"). `Link::step` is called by the deck
//! now and then (every few seconds while a code waits, then whenever the token is due); it answers what the deck shows:
//! `Pending { code }` (the splash shows the code and the QR of `pairing_url`) or `Ready { token }` (the API is open,
//! `Authorization: Bearer <token>`, read-only with the member's rights). Revoked or expired → the next step asks again
//! and answers `Pending` with a new code – the build never becomes useless. Network trouble is an `Err`: the caller
//! keeps playing what it has (built-in scenes, the cache) and steps again later.

use crate::handshake::{Ask, Handshake, Step};
pub use crate::handshake::{DeckInfo, State};
use crate::identity::Identity;
use anyhow::{anyhow, Result};
use serde_json::Value;
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

/// The blocking driver of the handshake (`Handshake`, the decisions shared with the browser deck): HTTP over the
/// transport, the signature from the deck's key file, the token's age on the deck's clock.
pub struct Link<T: Transport> {
    transport: T,
    identity: Identity,
    handshake: Handshake,
    token: Option<(String, String, Instant)>,
    renew_after: Duration,
}

impl<T: Transport> Link<T> {
    pub fn new(transport: T, identity: Identity, info: DeckInfo) -> Self {
        let handshake = Handshake::new(identity.public_key_hex(), info);
        Self {
            transport,
            identity,
            handshake,
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
        let mut ask = self.handshake.begin();
        // a round is a handful of asks (challenge, sign, token, perhaps a request and a second challenge)
        for _ in 0..8 {
            let step = match ask {
                Ask::Post { path, body } => {
                    let reply = self.transport.post(path, &body)?;
                    self.handshake.replied(reply.status, &reply.body).map_err(|e| anyhow!(e))?
                }
                Ask::Sign { message } => Step::Ask(
                    self.handshake
                        .signed(&self.identity.sign(message.as_bytes()))
                        .map_err(|e| anyhow!(e))?,
                ),
            };
            match step {
                Step::Ask(next) => ask = next,
                Step::Done(state) => {
                    if let State::Ready { token, name } = &state {
                        self.token = Some((token.clone(), name.clone(), now));
                    }
                    return Ok(state);
                }
            }
        }
        Err(anyhow!("the handshake did not end"))
    }
}
