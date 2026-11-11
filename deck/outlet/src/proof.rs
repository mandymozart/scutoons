//! A deck's proof for its live session (#478): the API's signed facts of this client (`GET /api/live/proof?session=<code>`:
//! a signed-in reader's account roles, an approved deck), which the relay client puts in its `hello.proof`. The browser deck
//! asks with its reader's sign-in, the native deck with its own token; where it asks and what the answer means is here, for
//! both. Without a proof a deck joins as a guest, as before.
use serde_json::Value;

/// how long after an answer without a proof the deck asks again (a reader may sign in, a deck be paired), seconds
pub const RETRY: f64 = 600.0;
/// a proof is asked again at half its life, never sooner than this, seconds
const SOONEST: f64 = 60.0;

/// the API's URL for this deck's proof in session `code`
pub fn request(api: &str, code: &str) -> String {
    let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    format!("{}/live/proof?session={code}", api.trim_end_matches('/'))
}

/// What the API's answer means for the relay client.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// a proof for every hello from now; asked again after `again` seconds
    Proof { proof: String, again: f64 },
    /// nobody signed in, no approved deck (401 / 403): no proof, a guest
    Guest,
    /// no answer, or one that says nothing: the proof it has stays until the next ask
    Failed,
}

/// The API's answer (`{ proof, expires_in }`, or `{ error }` with the status)
pub fn read(status: u16, body: &Value) -> Answer {
    match status {
        200 => match body["proof"].as_str().filter(|p| !p.is_empty()) {
            Some(p) => Answer::Proof { proof: p.to_string(), again: (body["expires_in"].as_f64().unwrap_or(0.0) / 2.0).max(SOONEST) },
            None => Answer::Failed,
        },
        401 | 403 => Answer::Guest,
        _ => Answer::Failed,
    }
}

/// When a deck asks: at once, then at half a proof's life, and `RETRY` after an answer without one – on the caller's clock
/// (seconds).
#[derive(Debug, Clone, PartialEq)]
pub struct Renew {
    due: f64,
}

impl Default for Renew {
    fn default() -> Self {
        Renew { due: f64::NEG_INFINITY }
    }
}

impl Renew {
    /// it is time to ask (once: then it waits for the answer)
    pub fn due(&mut self, now: f64) -> bool {
        let due = now >= self.due;
        if due {
            self.due = f64::INFINITY;
        }
        due
    }

    /// the answer came at `now`; what the relay client gets: `Some(proof)` to put in its hellos, `Some(None)` to drop the one
    /// it has, `None` to keep it
    pub fn answered(&mut self, now: f64, a: Answer) -> Option<Option<String>> {
        match a {
            Answer::Proof { proof, again } => {
                self.due = now + again;
                Some(Some(proof))
            }
            Answer::Guest => {
                self.due = now + RETRY;
                Some(None)
            }
            Answer::Failed => {
                self.due = now + RETRY;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn where_and_what() {
        assert_eq!(request("https://api.scutoons.com/api/", "A7K2"), "https://api.scutoons.com/api/live/proof?session=A7K2");
        assert_eq!(request("http://x/api", "a b&c=d"), "http://x/api/live/proof?session=abcd", "only a code's characters");
        assert_eq!(read(200, &json!({ "proof": "p.s", "expires_in": 3600 })), Answer::Proof { proof: "p.s".into(), again: 1800.0 });
        assert_eq!(read(200, &json!({ "proof": "p.s", "expires_in": 10 })), Answer::Proof { proof: "p.s".into(), again: 60.0 });
        assert_eq!(read(200, &json!({})), Answer::Failed);
        assert_eq!(read(401, &json!({ "error": "unauthorized" })), Answer::Guest);
        assert_eq!(read(403, &json!({ "error": "not_for_ai" })), Answer::Guest);
        assert_eq!(read(503, &Value::Null), Answer::Failed);
    }

    #[test]
    fn asks_at_once_then_at_half_its_life() {
        let mut r = Renew::default();
        assert!(r.due(0.0));
        assert!(!r.due(5.0), "asked: it waits for the answer");
        assert_eq!(r.answered(1.0, Answer::Proof { proof: "p".into(), again: 1800.0 }), Some(Some("p".into())));
        assert!(!r.due(1800.0));
        assert!(r.due(1801.0));
        assert_eq!(r.answered(1802.0, Answer::Failed), None, "offline: the proof it has stays");
        assert!(r.due(1802.0 + RETRY));
        assert_eq!(r.answered(2500.0, Answer::Guest), Some(None), "signed out: no proof");
    }
}
