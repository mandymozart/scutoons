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

/// The facts a proof states (its payload, `{ session, facts: { account, approved }, exp }`): the account roles of who
/// is at the deck – a signed-in reader's, or the member's a paired deck belongs to – and whether it is an approved deck.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub account: Vec<String>,
    pub approved: bool,
}

/// base64url without padding, as the API writes a proof's two parts
fn base64url(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut bits, mut have) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' => continue,
            _ => return None,
        };
        (bits, have) = ((bits << 6) | v as u32, have + 6);
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
        }
    }
    Some(out)
}

/// What `proof` states (#481: the deck reads its own facts – whether its person may reject a flash is one). The deck
/// cannot check the signature and need not: it has the proof from the API itself, over its own sign-in or token; the
/// relay checks it. None = no proof's shape.
pub fn facts(proof: &str) -> Option<Facts> {
    let payload = base64url(proof.split('.').next()?)?;
    let claims: Value = serde_json::from_slice(&payload).ok()?;
    let facts = claims.get("facts")?;
    let account = facts["account"].as_array().map(|a| a.iter().filter_map(|r| r.as_str().map(str::to_string)).collect()).unwrap_or_default();
    Some(Facts { account, approved: facts["approved"].as_bool().unwrap_or(false) })
}

/// The account roles an answer states for who is at the deck: `Some(roles)` to take from now (none for a guest), `None`
/// = the answer said nothing, what the deck knows stays.
pub fn account(a: &Answer) -> Option<Vec<String>> {
    match a {
        Answer::Proof { proof, .. } => Some(facts(proof).map(|f| f.account).unwrap_or_default()),
        Answer::Guest => Some(Vec::new()),
        Answer::Failed => None,
    }
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

    /// Ask again at once (who is signed in changed: a reader signed in or out)
    pub fn again(&mut self) {
        self.due = f64::NEG_INFINITY;
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
    fn a_proof_states_its_facts_to_the_deck_too() {
        // the vector the API signs and the relay verifies (deck/protocol/fixtures/live-proof.json)
        let vector: Value = serde_json::from_str(include_str!("../../protocol/fixtures/live-proof.json")).unwrap();
        let proof = vector["proof"].as_str().unwrap();
        assert_eq!(facts(proof), Some(Facts { account: vec!["member".into(), "live".into()], approved: true }));
        assert_eq!(account(&Answer::Proof { proof: proof.into(), again: 60.0 }), Some(vec!["member".to_string(), "live".to_string()]));
        // a guest has no account; no answer changes nothing; what is no proof states nothing
        assert_eq!((account(&Answer::Guest), account(&Answer::Failed)), (Some(vec![]), None));
        assert_eq!((facts("not a proof"), facts(""), facts("e30.x")), (None, None, None));
        assert_eq!(account(&Answer::Proof { proof: "e30.x".into(), again: 60.0 }), Some(vec![]));
    }

    #[test]
    fn asks_again_at_once_when_told() {
        let mut r = Renew::default();
        assert!(r.due(0.0));
        r.answered(1.0, Answer::Guest);
        assert!(!r.due(2.0));
        r.again();
        assert!(r.due(2.0));
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
