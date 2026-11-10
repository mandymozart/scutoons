//! A deck's link over time, sans-IO (#470): when the next handshake round is due, the token while it is fresh, the code
//! while a member is awaited, and the line the deck shows – the native daemon's schedule (main.rs: a code every 5 s, a
//! token renewed well within its hour, trouble again after 15 s) for a deck that drives the I/O itself (the browser deck:
//! the page's `fetch` and a non-extractable WebCrypto key). Times are the caller's clock in milliseconds.

use crate::handshake::{Ask, DeckInfo, Handshake, State, Step};
use crate::pairing::format_code;
use serde_json::Value;

/// while a member is awaited, the code is asked again after this
pub const POLL_MS: f64 = 5_000.0;
/// the token lives an hour (Decks::TOKEN_SECONDS); a new one is asked for after 50 minutes
pub const RENEW_MS: f64 = 50.0 * 60_000.0;
/// … and the old one is used no longer than 58 minutes (a renewal that fails leaves the deck without a token, not with a dead one)
pub const TOKEN_MS: f64 = 58.0 * 60_000.0;
/// after trouble (offline, a broken answer) the next round
pub const RETRY_MS: f64 = 15_000.0;

pub struct Session {
    handshake: Handshake,
    /// the token, the deck's name, when it came
    token: Option<(String, String, f64)>,
    code: Option<String>,
    problem: Option<String>,
    due: f64,
    busy: bool,
}

impl Session {
    pub fn new(public_key_hex: String, info: DeckInfo) -> Self {
        Self { handshake: Handshake::new(public_key_hex, info), token: None, code: None, problem: None, due: 0.0, busy: false }
    }

    /// The first ask of a round when one is due at `now` and none is running
    pub fn next(&mut self, now: f64) -> Option<Ask> {
        if self.busy || now < self.due {
            return None;
        }
        self.busy = true;
        Some(self.handshake.begin())
    }

    /// The answer to the last `Ask::Post` → the next ask, None = the round is over
    pub fn replied(&mut self, status: u16, body: &Value, now: f64) -> Option<Ask> {
        let step = self.handshake.replied(status, body);
        self.after(step, now)
    }

    /// The signature `Ask::Sign` wanted (hex) → the token request
    pub fn signed(&mut self, signature_hex: &str, now: f64) -> Option<Ask> {
        let step = self.handshake.signed(signature_hex).map(Step::Ask);
        self.after(step, now)
    }

    /// The round broke off (no answer, no key): tried again later; a fresh token stays
    pub fn failed(&mut self, why: &str, now: f64) {
        self.busy = false;
        self.problem = Some(why.to_string());
        self.due = now + RETRY_MS;
    }

    /// The API refused the token (401: revoked, ended): gone at once, a round right away
    pub fn forget(&mut self, now: f64) {
        self.token = None;
        if !self.busy {
            self.due = now;
        }
    }

    /// The token while it is fresh
    pub fn token(&self, now: f64) -> Option<&str> {
        self.token.as_ref().filter(|(_, _, at)| now - at < TOKEN_MS).map(|(t, _, _)| t.as_str())
    }

    /// The code to pair with (as the server gave it, K7Q42M) while a member is awaited
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Why there is no token ("" = there is one) – the reason a show medium is missing
    pub fn why(&self, now: f64) -> String {
        match (self.token(now), &self.code, &self.problem) {
            (Some(_), _, _) => String::new(),
            (None, Some(code), _) => format!("the deck is not paired – code {}", format_code(code)),
            (None, None, Some(p)) => format!("the deck link: {p}"),
            (None, None, None) => "the deck is not paired yet".into(),
        }
    }

    /// The deck's line about its link
    pub fn line(&self, now: f64) -> String {
        match (self.token.as_ref().filter(|_| self.token(now).is_some()), &self.code) {
            (Some((_, name, _)), _) if name.is_empty() => "DECK PAIRED".into(),
            (Some((_, name, _)), _) => format!("DECK PAIRED - {}", name.to_uppercase()),
            (None, Some(code)) => format!("PAIR THIS DECK - CODE {}", format_code(code)),
            (None, None) => self.problem.as_ref().map_or("DECK NOT PAIRED".into(), |p| format!("DECK LINK: {}", p.to_uppercase())),
        }
    }

    fn after(&mut self, step: Result<Step, String>, now: f64) -> Option<Ask> {
        match step {
            Ok(Step::Ask(a)) => Some(a),
            Ok(Step::Done(State::Ready { token, name })) => {
                self.token = Some((token, name, now));
                (self.code, self.problem, self.busy, self.due) = (None, None, false, now + RENEW_MS);
                None
            }
            Ok(Step::Done(State::Pending { code })) => {
                self.token = None;
                (self.code, self.problem, self.busy, self.due) = (Some(code), None, false, now + POLL_MS);
                None
            }
            Err(e) => {
                self.failed(&e, now);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn approved(s: &mut Session, now: f64) {
        assert!(matches!(s.next(now), Some(Ask::Post { path: "/decks/challenge", .. })));
        let sign = s.replied(200, &json!({ "deck": { "status": "approved", "challenge": "c" } }), now);
        assert!(matches!(sign, Some(Ask::Sign { .. })));
        assert!(matches!(s.signed(&"0".repeat(128), now), Some(Ask::Post { path: "/decks/token", .. })));
        assert_eq!(s.replied(200, &json!({ "token": "scd_t", "deck": { "name": "Foyer" } }), now), None);
    }

    #[test]
    fn a_waiting_deck_polls_its_code_and_then_holds_a_token_for_less_than_an_hour() {
        let mut s = Session::new("ab".into(), DeckInfo::default());
        assert_eq!(s.why(0.0), "the deck is not paired yet");
        s.next(0.0).unwrap();
        assert_eq!(s.next(1.0), None, "one round at a time");
        s.replied(401, &json!({ "error": "unknown_deck" }), 10.0).unwrap();
        assert_eq!(s.replied(201, &json!({ "deck": { "status": "pending", "code": "K7Q42M" } }), 20.0), None);
        assert_eq!((s.code(), s.why(20.0).as_str(), s.line(20.0).as_str()), (Some("K7Q42M"), "the deck is not paired – code K7Q-42M", "PAIR THIS DECK - CODE K7Q-42M"));
        assert_eq!(s.next(20.0 + POLL_MS - 1.0), None);
        // approved meanwhile
        approved(&mut s, 20.0 + POLL_MS);
        let t0 = 20.0 + POLL_MS;
        assert_eq!((s.token(t0), s.code(), s.why(t0).as_str(), s.line(t0).as_str()), (Some("scd_t"), None, "", "DECK PAIRED - FOYER"));
        assert_eq!(s.next(t0 + RENEW_MS - 1.0), None, "no round while the token is fresh");
        assert!(s.next(t0 + RENEW_MS).is_some(), "renewed after 50 minutes");
        // the renewal fails (offline): the old token holds until 58 minutes, then the deck has none
        s.failed("offline", t0 + RENEW_MS);
        assert_eq!(s.token(t0 + TOKEN_MS - 1.0), Some("scd_t"));
        assert_eq!(s.token(t0 + TOKEN_MS), None);
        assert_eq!(s.why(t0 + TOKEN_MS), "the deck link: offline");
        assert_eq!(s.next(t0 + RENEW_MS + RETRY_MS - 1.0), None);
        assert!(s.next(t0 + RENEW_MS + RETRY_MS).is_some());
    }

    #[test]
    fn a_refused_token_is_forgotten_at_once_and_a_broken_answer_waits() {
        let mut s = Session::new("ab".into(), DeckInfo::default());
        approved(&mut s, 0.0);
        s.forget(100.0);
        assert_eq!(s.token(100.0), None);
        assert!(s.next(100.0).is_some(), "a round right away");
        assert_eq!(s.replied(500, &json!({}), 110.0), None);
        assert_eq!(s.line(110.0), "DECK LINK: CHALLENGE: HTTP 500");
        assert_eq!(s.next(110.0), None);
        assert!(s.next(110.0 + RETRY_MS).is_some());
    }
}
