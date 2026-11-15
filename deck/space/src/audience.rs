//! The audience start (#549, research/SHOW.md §15 point 3b; owner 2026-11-14 [live]: "Deck draws the questions … only
//! stuff that is not accessible through the deck's api on the phone's hardware we will build bridges"): a deck that is only
//! there to vote – a phone that scanned the room's QR. It holds its own relay client (relay.rs): role `audience`, the QR's
//! group as the group it came with, `device: phone`, its proof asked as an outlet's is (deck-outlet proof.rs: `r:`, the
//! page fetches it with the reader's sign-in – a signed-in phone is a member at the relay, a signed-out one a guest). No
//! outlet, no cells, no scene: the vote the relay sends it (#547: only to who may vote) is its ballot (deck-ui ballot.rs,
//! #556), between votes it says where it is and that it waits (`ballot::waiting`); a press or a key casts through the
//! relay client. The platform brings the socket, the proof's fetch and a screen for the layer (stage/light.rs), and asks
//! `awake` whether to keep the screen on.
use crate::relay::{Client, Options, Out};
use deck_outlet::proof::{self, Renew};
use deck_ui::ballot::{self, Page, View};
use deck_ui::live::Do;
pub use deck_ui::menu::Input;
use protocol::Role;

/// what an audience deck says it is (`hello.device`; a vote's filter asks it: `where: { device: phone }`)
pub const DEVICE: &str = "phone";

pub struct Audience {
    pub relay: Client,
    /// the API base: where the proof is asked
    api: String,
    /// the ballot's words
    locale: String,
    /// when it asks for its proof (`r:`)
    renew: Renew,
    /// what the page should fetch: key and URL
    asks: Vec<(String, String)>,
    page: Page,
}

impl Audience {
    /// Join `relay` (ws:// or wss://…/live/<code>; "" = the hosted show) as an audience phone that came with `group` (the
    /// QR's `?group=`; "" = none), named after `reader` (the signed-in reader's name; "" = nobody), its words in `locale`,
    /// its proof from `api`.
    pub fn new(relay: &str, group: &str, reader: &str, locale: &str, api: &str) -> Result<Audience, String> {
        let relay = if relay.trim().is_empty() { crate::relay::HOSTED_RELAY } else { relay.trim() };
        let group = group.trim();
        if !group.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!("group '{group}': letters, digits, - and _ only"));
        }
        let mut o = Options::new(relay, Role::Audience, Some(group).filter(|g| !g.is_empty()));
        // a phone that lost its link comes back as the same client: its vote stays its one vote
        o.resume = true;
        o.nick = Some(deck_ui::live::preset_name(None, Some(reader), DEVICE));
        o.device = Some(DEVICE.into());
        // the first hello waits for the proof (`take` asks for it at once, `proof` hands the answer on)
        o.wait_proof = true;
        Ok(Audience { relay: Client::new(o)?, api: api.trim_end_matches('/').to_owned(), locale: if locale.is_empty() { "de".into() } else { locale.to_owned() }, renew: Renew::default(), asks: Vec::new(), page: Page::default() })
    }

    /// What the relay client asks of the page's socket, at `now` (the page's clock, s). A vote is the client's ballot:
    /// the page gets nothing of it.
    pub fn take(&mut self, now: f64) -> Vec<Out> {
        if self.renew.due(now) {
            self.asks.push(("r:".to_string(), proof::request(&self.api, self.relay.code())));
        }
        self.relay.take().into_iter().filter(|o| !matches!(o, Out::Vote(_) | Out::Message(_))).collect()
    }

    /// what the page should fetch: key (`r:` the proof, with the reader's sign-in) and URL
    pub fn asks(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.asks)
    }

    /// The API's answer to the `r:` ask (`status` 0: no answer) at `now` (s) – the proof for the relay client's hellos
    pub fn proof(&mut self, status: u16, body: &str, now: f64) {
        let body = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        self.relay.proof_answered(self.renew.answered(now, proof::read(status, &body)));
    }

    /// who is signed in changed (the page says so): the proof is asked again at once
    pub fn proof_again(&mut self) {
        self.renew.again();
    }

    /// What shows at `now` (the relay client's clock: the socket's ms): the ballot of the vote this phone may cast in,
    /// else where it is and that it waits.
    pub fn view(&self, now: f64) -> View {
        self.relay.ballot().view(now, &self.locale).unwrap_or_else(|| ballot::waiting(&self.relay.live(), &self.locale))
    }

    /// The screen's layer at `now` for a `size` screen (device pixels) in the deck's mono face: None = as before.
    pub fn paint(&mut self, now: f64, size: (f32, f32), per_px: f32) -> Option<Option<deck_kit::base::screen::Layer>> {
        let view = self.view(now);
        self.page.paint(Some(&view), size, per_px)
    }

    /// the screen was made anew: painted again with the next `paint`
    pub fn again(&mut self) {
        self.page.again();
    }

    /// One input of the person (a press, a key) at `now`: the ballot's – a cast goes to the relay client. Whether it took it.
    pub fn input(&mut self, i: Input, now: f64) -> bool {
        let view = self.view(now);
        let (took, did) = self.page.input(i, Some(&view));
        if let Some(Do::Cast(option)) = did {
            self.relay.vote_cast(option as i64);
        }
        took
    }

    /// The middle of option `i` as painted, in device pixels (a scripted press); none = not there
    pub fn at(&self, i: usize) -> Option<(f32, f32)> {
        self.page.at(i)
    }

    /// A vote can come: it is in its session – the page keeps the screen on while this holds (the wake lock is the
    /// platform's, when to hold it is said here).
    pub fn awake(&self) -> bool {
        self.relay.joined()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Envelope, Message};
    use serde_json::json;

    const SIZE: (f32, f32) = (390.0, 844.0);

    fn says(kind: &str, p: serde_json::Value) -> String {
        let message: Message = serde_json::from_value(json!({ "kind": kind, "p": p })).expect("a message");
        serde_json::to_string(&Envelope::new("relay", message)).unwrap()
    }

    /// the hello among what the client asks to send
    fn hello(out: &[Out]) -> Option<protocol::Hello> {
        out.iter().find_map(|o| match o {
            Out::Send(text) => match serde_json::from_str::<Envelope>(text).ok()?.message {
                Message::Hello(h) => Some(h),
                _ => None,
            },
            _ => None,
        })
    }

    fn texts(a: &mut Audience, now: f64) -> Vec<String> {
        a.again();
        a.paint(now, SIZE, 0.6).flatten().map(|l| l.list.texts().into_iter().map(String::from).collect()).unwrap_or_default()
    }

    /// to the open socket: started, the proof answered, dialled, opened – what it sent
    fn open(a: &mut Audience, proof: (u16, &str)) -> Vec<Out> {
        a.relay.start(0.0);
        assert_eq!(a.take(0.0).iter().filter(|o| matches!(o, Out::Dial(_))).count(), 0, "the first dial waits for the proof");
        assert_eq!(a.asks(), [("r:".to_string(), "https://api.test/api/live/proof?session=CATH".to_string())]);
        a.proof(proof.0, proof.1, 0.1);
        a.relay.tick(100.0);
        let dial = a.take(0.2);
        assert!(dial.iter().any(|o| matches!(o, Out::Dial(url) if url.contains("role=audience"))), "{dial:?}");
        a.relay.opened(150.0);
        a.take(0.3)
    }

    #[test]
    fn a_phone_joins_as_audience_of_its_group_with_its_device_and_its_proof() {
        let mut a = Audience::new("ws://relay.test:7070/live/CATH", "left", "Anna", "en", "https://api.test/api/").unwrap();
        let h = hello(&open(&mut a, (200, r#"{"proof":"p.s","expires_in":3600}"#))).expect("the hello");
        assert_eq!((h.role, h.group.as_deref(), h.device.as_deref(), h.nick.as_deref(), h.proof.as_deref()), (Role::Audience, Some("left"), Some("phone"), Some("Anna"), Some("p.s")));
        // nobody signed in: the same hello without a proof – a guest; no group in the QR: none, the name the device's
        let mut g = Audience::new("ws://relay.test:7070/live/CATH", "", "", "en", "https://api.test/api").unwrap();
        let h = hello(&open(&mut g, (401, r#"{"error":"unauthorized"}"#))).expect("the hello");
        assert_eq!((h.role, h.group, h.device.as_deref(), h.nick.as_deref(), h.proof), (Role::Audience, None, Some("phone"), Some("phone"), None));
        assert!(Audience::new("ws://relay.test:7070/live/CATH", "le ft", "", "en", "").is_err(), "a group is a role's id");
        assert!(Audience::new("no relay", "left", "", "en", "").is_err());
    }

    #[test]
    fn it_waits_then_draws_the_vote_casts_with_a_press_and_shows_the_result() {
        let mut a = Audience::new("ws://relay.test:7070/live/CATH", "left", "", "en", "https://api.test/api").unwrap();
        assert_eq!(texts(&mut a, 0.0), ["LEFT", "CATH", "JOINING CATH"], "not in its session yet: the link's line");
        assert!(!a.awake() && !a.input(Input::Enter, 0.0));
        open(&mut a, (0, ""));
        a.relay.text(200.0, &says("welcome", json!({ "client": "c7", "snapshot": {}, "now": 0 })));
        assert!(a.awake(), "in its session: a vote can come");
        assert_eq!(texts(&mut a, 300.0), ["LEFT", "CATH", "Waiting for the next vote."]);
        let vote = json!({ "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20, "show": true, "where": { "device": "phone" }, "question": { "en": "Shall the cat sing?" } });
        a.relay.text(1000.0, &says("vote.open", vote));
        assert_eq!(a.take(1.0).iter().filter(|o| matches!(o, Out::Vote(_) | Out::Message(_))).count(), 0, "the page hears nothing of a vote");
        assert_eq!(texts(&mut a, 2000.0), ["VOTE", "19 S LEFT", "Shall the cat sing?", "Yes", "", "No", "", "Make your choice."]);
        // a tap on the second option casts it
        let (x, y) = a.at(1).expect("the second option");
        assert!(a.input(Input::Press(x, y), 2500.0));
        let cast = a.take(2.5).into_iter().find_map(|o| match o {
            Out::Send(text) => match serde_json::from_str::<Envelope>(&text).ok()?.message {
                Message::VoteCast(c) => Some((c.vote, c.option)),
                _ => None,
            },
            _ => None,
        });
        assert_eq!(cast, Some(("v1".to_string(), 1)));
        a.relay.text(3000.0, &says("vote.tally", json!({ "vote": "v1", "counts": [1, 3], "voters": 4, "left": 17000 })));
        assert_eq!(texts(&mut a, 3000.0), ["VOTE", "17 S LEFT", "Shall the cat sing?", "Yes", "25%", "No", "75%", "You can change it until the end."]);
        a.relay.text(4000.0, &says("vote.result", json!({ "vote": "v1", "option": 1, "counts": [1, 3], "how": "rule" })));
        assert_eq!(texts(&mut a, 4500.0), ["RESULT", "", "No", "Most votes.", "Your vote: No"]);
        assert_eq!(texts(&mut a, 4000.0 + ballot::RESULT_MS), ["LEFT", "CATH", "Waiting for the next vote."], "the result has shown: it waits again");
        // the link is lost: no vote can come until it is back
        a.relay.closed(20_000.0, "gone");
        assert!(!a.awake());
    }

    #[test]
    fn the_proof_is_asked_again_when_the_sign_in_changes() {
        let mut a = Audience::new("ws://relay.test:7070/live/CATH", "left", "", "en", "https://api.test/api").unwrap();
        open(&mut a, (401, ""));
        a.take(5.0);
        assert_eq!(a.asks(), [], "answered: not again for now");
        a.proof_again();
        a.take(6.0);
        assert_eq!(a.asks().len(), 1);
    }
}
