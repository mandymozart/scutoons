//! One round of the handshake as a sans-IO machine (#470): request → challenge → signature → token
//! (CYBERDECKS.md §9.2–9.5; server/docs/API.md "Decks"). It says what to do next (`Ask`: POST this body, sign this
//! message) and reads the answers; the caller does the I/O and keeps the token's age. The native daemon drives it with
//! blocking HTTP and the deck's key file (`Link`), the browser deck with `fetch` and a non-extractable WebCrypto key –
//! the same decisions on both, and the private key never passes through here (only the message to sign and its signature).

use serde_json::{json, Value};

/// What the deck says about itself in its request (shown to the member before approving)
#[derive(Debug, Clone, Default)]
pub struct DeckInfo {
    pub build: String,
    pub class: String,
    pub hostname: String,
}

impl DeckInfo {
    /// The class from deck-check's machine.toml (`class = "A"`, deck/os/check) – empty when there is none
    pub fn class_of_machine(toml: &str) -> String {
        toml.lines()
            .filter_map(|l| l.trim().strip_prefix("class"))
            .filter_map(|rest| rest.trim_start().strip_prefix('='))
            .map(|v| v.trim().trim_matches('"').to_string())
            .next()
            .unwrap_or_default()
    }
}

/// What the deck shows after a round
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// waiting for a member: show the code (`format_code`) and the QR of `pairing_url`
    Pending { code: String },
    /// approved: the token for the API, and the name the member gave the deck
    Ready { token: String, name: String },
}

/// The next thing to do for the handshake
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// POST `body` (JSON) to `path` below the API base (`/decks/request` …), then `replied` with the status and body
    Post { path: &'static str, body: Value },
    /// sign `message` (UTF-8 bytes) with the deck's Ed25519 key, then `signed` with the 64-byte signature as hex
    Sign { message: String },
}

/// After an answer: the next ask, or the round's end
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Ask(Ask),
    Done(State),
}

/// What the deck signs (server Decks::SIGNED_PREFIX): bound to this protocol, the challenge and the deck's key
pub fn signed_message(challenge: &str, public_key_hex: &str) -> String {
    format!("scutoons-deck-token:v1:{challenge}:{public_key_hex}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Idle,
    Challenge,
    Sign(String),
    Token,
    Request,
}

/// One handshake round for a deck's public key (32 bytes, lower-case hex)
pub struct Handshake {
    public_key_hex: String,
    info: DeckInfo,
    phase: Phase,
    /// requests that came back `approved` this round (the server then wants its challenge; twice = a broken server)
    approved: u8,
}

impl Handshake {
    pub fn new(public_key_hex: String, info: DeckInfo) -> Self {
        Self { public_key_hex, info, phase: Phase::Idle, approved: 0 }
    }

    pub fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    /// A new round (also after an error): ask for a challenge first – an approved deck gets one, a waiting one its code
    pub fn begin(&mut self) -> Ask {
        self.approved = 0;
        self.challenge()
    }

    /// The answer to the last `Ask::Post`. Err = a broken or unexpected answer (the caller retries later with `begin`).
    pub fn replied(&mut self, status: u16, body: &Value) -> Result<Step, String> {
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        match phase {
            Phase::Challenge => {
                if status == 401 {
                    return Ok(Step::Ask(self.request()));
                }
                let deck = expect(status, body, &[200, 202], "challenge")?;
                match deck["status"].as_str() {
                    Some("pending") => Ok(Step::Done(State::Pending { code: text(&deck["code"], "code")? })),
                    Some("approved") => {
                        let challenge = text(&deck["challenge"], "challenge")?;
                        let message = signed_message(&challenge, &self.public_key_hex);
                        self.phase = Phase::Sign(challenge);
                        Ok(Step::Ask(Ask::Sign { message }))
                    }
                    other => Err(format!("unexpected challenge status {other:?}")),
                }
            }
            Phase::Token => {
                if status == 401 {
                    let code = body["error"].as_str().unwrap_or("");
                    // the approval ended meanwhile: ask again (a new code)
                    if matches!(code, "revoked" | "expired" | "unknown_deck" | "not_approved" | "request_expired") {
                        return Ok(Step::Ask(self.request()));
                    }
                    return Err(format!("token refused: {code}"));
                }
                if status != 200 {
                    return Err(format!("token: HTTP {status}"));
                }
                let name = body["deck"]["name"].as_str().unwrap_or("").to_string();
                Ok(Step::Done(State::Ready { token: text(&body["token"], "token")?, name }))
            }
            Phase::Request => {
                let deck = expect(status, body, &[200, 201], "request")?;
                match deck["status"].as_str() {
                    Some("pending") => Ok(Step::Done(State::Pending { code: text(&deck["code"], "code")? })),
                    Some("approved") => {
                        self.approved += 1;
                        if self.approved >= 2 {
                            return Err("the server keeps answering approved without a challenge".into());
                        }
                        Ok(Step::Ask(self.challenge()))
                    }
                    other => Err(format!("unexpected request status {other:?}")),
                }
            }
            Phase::Idle | Phase::Sign(_) => Err("no request is waiting for an answer".into()),
        }
    }

    /// The signature of the message `Ask::Sign` named (64 bytes, lower-case hex) → the token request
    pub fn signed(&mut self, signature_hex: &str) -> Result<Ask, String> {
        let Phase::Sign(challenge) = std::mem::replace(&mut self.phase, Phase::Idle) else {
            return Err("nothing is waiting for a signature".into());
        };
        let ok = signature_hex.len() == 128 && signature_hex.bytes().all(|b| b.is_ascii_hexdigit());
        if !ok {
            return Err("a signature is 64 bytes as hex".into());
        }
        self.phase = Phase::Token;
        Ok(Ask::Post {
            path: "/decks/token",
            body: json!({
                "public_key": self.public_key_hex,
                "challenge": challenge,
                "signature": signature_hex.to_ascii_lowercase(),
            }),
        })
    }

    fn challenge(&mut self) -> Ask {
        self.phase = Phase::Challenge;
        Ask::Post { path: "/decks/challenge", body: json!({ "public_key": self.public_key_hex }) }
    }

    fn request(&mut self) -> Ask {
        self.phase = Phase::Request;
        Ask::Post {
            path: "/decks/request",
            body: json!({
                "public_key": self.public_key_hex,
                "build": self.info.build,
                "class": self.info.class,
                "hostname": self.info.hostname,
            }),
        }
    }
}

fn expect<'a>(status: u16, body: &'a Value, ok: &[u16], what: &str) -> Result<&'a Value, String> {
    if !ok.contains(&status) {
        let code = body["error"].as_str().unwrap_or("");
        return Err(format!("{what}: HTTP {status} {code}").trim_end().to_string());
    }
    Ok(&body["deck"])
}

fn text(value: &Value, what: &str) -> Result<String, String> {
    value.as_str().map(str::to_string).ok_or_else(|| format!("the answer has no {what}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PK: &str = "ab";

    fn post(ask: &Ask) -> &'static str {
        match ask {
            Ask::Post { path, .. } => path,
            Ask::Sign { .. } => panic!("a post was expected: {ask:?}"),
        }
    }

    fn step_ask(step: Result<Step, String>) -> Ask {
        match step.unwrap() {
            Step::Ask(a) => a,
            Step::Done(s) => panic!("an ask was expected: {s:?}"),
        }
    }

    #[test]
    fn the_class_comes_from_machine_toml() {
        let toml = "# written by deck-check\nhostname = \"x1\"\nclassic = \"no\"\nclass = \"A\"\n[features]\nclass = \"B\"\n";
        assert_eq!(DeckInfo::class_of_machine(toml), "A");
        assert_eq!(DeckInfo::class_of_machine("hostname = \"x1\"\n"), "");
    }

    #[test]
    fn an_unknown_deck_requests_and_shows_its_code() {
        let mut h = Handshake::new(PK.into(), DeckInfo { build: "s7".into(), class: "web".into(), hostname: "browser".into() });
        assert_eq!(post(&h.begin()), "/decks/challenge");
        let req = step_ask(h.replied(401, &json!({ "error": "unknown_deck" })));
        let Ask::Post { path, body } = &req else { panic!() };
        assert_eq!((*path, body["class"].as_str(), body["public_key"].as_str()), ("/decks/request", Some("web"), Some(PK)));
        let done = h.replied(201, &json!({ "deck": { "status": "pending", "code": "K7Q42M" } })).unwrap();
        assert_eq!(done, Step::Done(State::Pending { code: "K7Q42M".into() }));
    }

    #[test]
    fn an_approved_deck_signs_its_challenge_and_gets_a_token() {
        let mut h = Handshake::new(PK.into(), DeckInfo::default());
        h.begin();
        let sign = step_ask(h.replied(200, &json!({ "deck": { "status": "approved", "challenge": "c0ffee" } })));
        assert_eq!(sign, Ask::Sign { message: format!("scutoons-deck-token:v1:c0ffee:{PK}") });
        assert!(h.signed("xyz").is_err(), "not a signature");
        h.begin();
        step_ask(h.replied(200, &json!({ "deck": { "status": "approved", "challenge": "c0ffee" } })));
        let token = h.signed(&"AB".repeat(64)).unwrap();
        let Ask::Post { path, body } = &token else { panic!() };
        assert_eq!((*path, body["challenge"].as_str(), body["signature"].as_str()), ("/decks/token", Some("c0ffee"), Some("ab".repeat(64).as_str())));
        let done = h.replied(200, &json!({ "token": "scd_t", "deck": { "name": "Stage left" } })).unwrap();
        assert_eq!(done, Step::Done(State::Ready { token: "scd_t".into(), name: "Stage left".into() }));
    }

    #[test]
    fn a_revoked_approval_asks_again_and_a_broken_server_is_an_error() {
        let mut h = Handshake::new(PK.into(), DeckInfo::default());
        h.begin();
        step_ask(h.replied(200, &json!({ "deck": { "status": "approved", "challenge": "c" } })));
        h.signed(&"0".repeat(128)).unwrap();
        assert_eq!(post(&step_ask(h.replied(401, &json!({ "error": "revoked" })))), "/decks/request");
        // approved twice without a challenge: an error, not a loop
        assert_eq!(post(&step_ask(h.replied(200, &json!({ "deck": { "status": "approved" } })))), "/decks/challenge");
        assert_eq!(post(&step_ask(h.replied(401, &json!({}))) ), "/decks/request");
        assert!(h.replied(200, &json!({ "deck": { "status": "approved" } })).is_err());
        // other failures
        h.begin();
        assert!(h.replied(500, &json!({})).is_err());
        assert!(h.replied(200, &json!({})).is_err(), "nothing is waiting");
        assert!(h.signed(&"0".repeat(128)).is_err(), "nothing to sign");
        h.begin();
        step_ask(h.replied(200, &json!({ "deck": { "status": "approved", "challenge": "c" } })));
        h.signed(&"0".repeat(128)).unwrap();
        assert_eq!(h.replied(401, &json!({ "error": "bad_signature" })), Err("token refused: bad_signature".into()));
    }
}
