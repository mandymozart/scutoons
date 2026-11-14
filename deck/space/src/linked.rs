//! The linked reader (#527, agents/research/SCENE-EVENTS.md §6): a reader page linked to a live session drives the decks'
//! story, one reader at a time – the newest claim wins, the relay's rule (relay session.rs, `reader.drive`). Here, once for
//! every client of the core (RULES A6): the link a conductor offers (`reader_link` – the deck editor and the conductor
//! page give the same text), a reader's place in the story (`Steer`, driven by relay.rs `Client::drive`) and the
//! conductor's view of the linked readers (`Readers`, relay.rs `Client::readers`).
use crate::relay::{encode, parse_relay, HOSTED_RELAY};
use protocol::{Loose, Presence};
use serde::Serialize;
use std::collections::BTreeMap;

/// The address a linked reader opens: the viewer's `base` (its origin, a locale prefix if any), the toon, the chapter's
/// number, the label to start at (none or empty: the chapter's start), the session's code and its relay (#530: the URL the
/// conductor joined, `ws://host:port/live/<code>`) – `<base>/<toon>/chapter/<n>?label=<label>&live=<code>&relay=<relay>`,
/// every part escaped. The relay goes in as its base (the reader adds `/live/<code>`, viewer live-deck.ts `relayOf`) and
/// only when it is not the hosted one (`HOSTED_RELAY`, the reader's default); none, empty or no relay URL: none.
pub fn reader_link(base: &str, toon: &str, chapter: u32, label: Option<&str>, code: &str, relay: Option<&str>) -> String {
    let label = label.map(str::trim).filter(|l| !l.is_empty()).map(|l| format!("label={}&", encode(l))).unwrap_or_default();
    let hosted = parse_relay(HOSTED_RELAY).map(|(b, _)| b).unwrap_or_default();
    let relay = relay
        .and_then(|r| parse_relay(r).ok())
        .map(|(b, _)| b)
        .filter(|b| !b.eq_ignore_ascii_case(&hosted))
        .map(|b| format!("&relay={}", encode(&b)))
        .unwrap_or_default();
    format!("{}/{}/chapter/{chapter}?{label}live={}{relay}", base.trim().trim_end_matches('/'), encode(toon.trim()), encode(code.trim()))
}

/// Where a reader stands in its session's story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Claim {
    /// it has not asked to drive: its story goes nowhere
    #[default]
    None,
    /// it asked (`reader.drive`) and waits for the relay's word: its story is dropped, not held
    Asked,
    /// it drives: its story goes to the conductor and the decks that asked
    Drives,
    /// a newer reader took over: its story is dropped; asking again takes the place back
    TakenOver,
}

/// A reader's place as its page hears it (relay.rs `Out::Driving`, `Client::driving`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Driving {
    pub claim: Claim,
    /// another reader drives now – a claim would take it over (the newcomer's warning, owner 2026-11-13 [scenes])
    pub other: bool,
}

/// A reader's claim and the relay's word on who drives (relay.rs keeps one per client).
#[derive(Debug, Clone, Default)]
pub struct Steer {
    claim: Claim,
    driver: Option<String>,
    told: Option<Driving>,
}

impl Steer {
    /// the person said yes: it drives from the relay's answer on (the caller sends `reader.drive` once joined)
    pub fn claim(&mut self) {
        self.claim = Claim::Asked;
    }

    /// a claim waits to go out (at the welcome)
    pub fn asked(&self) -> bool {
        self.claim == Claim::Asked
    }

    /// its story may go out now
    pub fn drives(&self) -> bool {
        self.claim == Claim::Drives
    }

    /// a welcome as `me`, with who drives now (its snapshot's `driver`). True: claim again – it had asked, or it drove
    /// and nobody took the place while it was away (a driver that leaves frees it); someone did: taken over.
    pub fn welcome(&mut self, me: &str, driver: Option<String>) -> bool {
        let free = driver.as_deref().is_none_or(|d| d == me);
        self.driver = driver;
        match self.claim {
            Claim::Asked => true,
            Claim::Drives if free => {
                self.claim = Claim::Asked;
                true
            }
            Claim::Drives => {
                self.claim = Claim::TakenOver;
                false
            }
            Claim::None | Claim::TakenOver => false,
        }
    }

    /// the relay's word as `me`: who drives now – its own id grants a claim; another's takes the place of a driver
    pub fn heard(&mut self, me: &str, driver: Option<String>) {
        match (self.claim, driver.as_deref()) {
            (Claim::Asked, Some(d)) if d == me => self.claim = Claim::Drives,
            (Claim::Drives, Some(d)) if d != me => self.claim = Claim::TakenOver,
            _ => {}
        }
        self.driver = driver;
    }

    pub fn now(&self, me: &str) -> Driving {
        Driving { claim: self.claim, other: self.driver.as_deref().is_some_and(|d| d != me) }
    }

    /// what changed since it was last told (relay.rs `Out::Driving`)
    pub fn change(&mut self, me: &str) -> Option<Driving> {
        let now = self.now(me);
        (self.told != Some(now)).then(|| {
            self.told = Some(now);
            now
        })
    }
}

/// The last story event the conductor heard: its type, the characters it names (references, `character:<id>`), the reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Heard {
    #[serde(rename = "type")]
    pub kind: String,
    pub characters: Vec<String>,
    pub from: String,
}

/// The conductor's view of the linked readers: who is in by name (presence), which one drives, the last story heard.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Readers {
    /// the linked readers now, by client id: the name each said (none: it said none)
    pub linked: BTreeMap<String, Option<String>>,
    pub driver: Option<String>,
    pub last: Option<Heard>,
}

impl Readers {
    /// the relay's presence: the readers in the session and who drives
    pub fn presence(&mut self, p: &Presence) {
        self.linked = p.readers.iter().flatten().map(|(id, r)| (id.clone(), r.nick.clone())).collect();
        self.driver.clone_from(&p.driver);
    }

    /// a story event a reader sent (the relay passes only the driver's): its type and the characters it names, read as a
    /// listening scene reads them (scene_play.rs)
    pub fn heard(&mut self, from: &str, event: &Loose) {
        let kind = event.get("type").and_then(|t| t.as_str()).unwrap_or_default().to_string();
        let characters = crate::scene_play::characters(event.get("params"));
        self.last = Some(Heard { kind, characters, from: from.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_link_a_conductor_offers() {
        assert_eq!(reader_link("https://scutoons.com", "catpope", 3, None, "A7K2", None), "https://scutoons.com/catpope/chapter/3?live=A7K2");
        assert_eq!(reader_link("https://scutoons.com/de/", "catpope", 3, Some("dialog-start"), "A7K2", None), "https://scutoons.com/de/catpope/chapter/3?label=dialog-start&live=A7K2");
        assert_eq!(reader_link("https://scutoons.com", "catpope", 1, Some("  "), "A7K2", None), "https://scutoons.com/catpope/chapter/1?live=A7K2", "an empty label: the chapter's start");
        // escaped: a label with spaces, `&` and umlauts, a code and a toon that would break the address
        assert_eq!(
            reader_link("http://192.168.1.116:4100", "cat pope", 2, Some("Grüße & co/2"), "A B&c=1", None),
            "http://192.168.1.116:4100/cat%20pope/chapter/2?label=Gr%C3%BC%C3%9Fe%20%26%20co%2F2&live=A%20B%26c%3D1"
        );
    }

    #[test]
    fn the_link_names_the_relay_only_when_it_is_not_the_hosted_one() {
        let link = |relay| reader_link("https://scutoons.com", "catpope", 3, Some("dialog-start"), "A7K2", relay);
        let today = "https://scutoons.com/catpope/chapter/3?label=dialog-start&live=A7K2";
        // the hosted relay (the reader's default) – any session, the host in any case, with a query: the link as before
        assert_eq!(link(Some("wss://live.scutoons.com/live/A7K2")), today);
        assert_eq!(link(Some("wss://Live.Scutoons.com/live/A7K2/?quic=")), today);
        assert_eq!(link(Some(HOSTED_RELAY)), today);
        assert_eq!(link(None), today);
        assert_eq!(link(Some("  ")), today, "an empty relay: none");
        assert_eq!(link(Some("http://192.168.1.116:7070/live/A7K2")), today, "no ws:// relay: none");
        // another relay: its base (the reader adds `/live/<code>`), escaped, without the query (the QUIC door)
        assert_eq!(link(Some("ws://192.168.1.116:7070/live/A7K2?quic=192.168.1.116:7071")), format!("{today}&relay=ws%3A%2F%2F192.168.1.116%3A7070"));
        assert_eq!(link(Some("wss://scutoons.test/relay/live/A7K2")), format!("{today}&relay=wss%3A%2F%2Fscutoons.test%2Frelay"), "a path before /live/ stays");
        assert_eq!(
            reader_link("https://scutoons.com", "catpope", 3, None, "A7K2", Some("ws://127.0.0.1:7070/live/A7K2")),
            "https://scutoons.com/catpope/chapter/3?live=A7K2&relay=ws%3A%2F%2F127.0.0.1%3A7070"
        );
    }

    fn steer() -> Steer {
        let mut s = Steer::default();
        s.claim();
        s
    }

    #[test]
    fn a_reader_drives_from_the_relay_s_word_and_is_taken_over() {
        let mut s = Steer::default();
        assert_eq!(s.now("c4"), Driving::default());
        assert!(!s.welcome("c4", Some("c2".into())), "not asked: no claim");
        assert_eq!(s.now("c4"), Driving { claim: Claim::None, other: true }, "a newcomer sees another drives");
        s.claim();
        assert!(s.asked() && !s.drives());
        s.heard("c4", Some("c2".into()));
        assert!(s.asked(), "another's word is no grant");
        s.heard("c4", Some("c4".into()));
        assert!(s.drives());
        assert_eq!(s.now("c4"), Driving { claim: Claim::Drives, other: false });
        s.heard("c4", None);
        assert!(s.drives(), "only another's claim takes the place");
        s.heard("c4", Some("c9".into()));
        assert_eq!(s.now("c4"), Driving { claim: Claim::TakenOver, other: true });
        s.claim();
        s.heard("c4", Some("c4".into()));
        assert!(s.drives(), "asking again takes it back");
    }

    #[test]
    fn a_driver_back_from_a_drop_claims_again_unless_another_took_the_place() {
        let mut s = steer();
        assert!(s.welcome("c4", None), "the first welcome sends the claim");
        s.heard("c4", Some("c4".into()));
        assert!(s.welcome("c5", None), "back as c5, the place free: claims again");
        assert!(s.asked(), "and waits for the word");
        s.heard("c5", Some("c5".into()));
        assert!(!s.welcome("c6", Some("c7".into())), "c7 claimed while it was away");
        assert_eq!(s.now("c6").claim, Claim::TakenOver);
        let mut asked = steer();
        assert!(asked.welcome("c4", Some("c7".into())), "a claim not yet answered goes again: the newest wins");
    }

    #[test]
    fn change_tells_each_new_state_once() {
        let mut s = Steer::default();
        assert_eq!(s.change("c4"), Some(Driving::default()));
        assert_eq!(s.change("c4"), None);
        s.claim();
        assert_eq!(s.change("c4").map(|d| d.claim), Some(Claim::Asked));
        assert_eq!(s.change("c4"), None);
    }

    #[test]
    fn the_conductor_sees_the_linked_readers_and_the_last_story() {
        let mut r = Readers::default();
        let p: Presence = serde_json::from_value(json!({ "groups": {}, "ready": [], "readers": { "c4": { "nick": "Mira" }, "c5": {} }, "driver": "c4" })).unwrap();
        r.presence(&p);
        assert_eq!(r.linked, BTreeMap::from([("c4".to_string(), Some("Mira".to_string())), ("c5".to_string(), None)]));
        assert_eq!(r.driver.as_deref(), Some("c4"));
        let join = json!({ "type": "joinScene", "params": { "characters": [{ "character": "catpope" }, { "character": "yellowmouse", "persona": "kid" }] }, "id": "e1" });
        r.heard("c4", join.as_object().unwrap());
        assert_eq!(r.last, Some(Heard { kind: "joinScene".into(), characters: vec!["character:catpope".into(), "character:yellowmouse.kid".into()], from: "c4".into() }));
        let say = json!({ "type": "showMessage", "params": { "character": "catpope", "text": { "de": "Hallo" } } });
        r.heard("c4", say.as_object().unwrap());
        assert_eq!(r.last.as_ref().map(|h| (h.kind.as_str(), h.characters.clone())), Some(("showMessage", vec!["character:catpope".to_string()])));
        r.presence(&serde_json::from_value(json!({ "groups": {}, "ready": [] })).unwrap());
        assert!(r.linked.is_empty() && r.driver.is_none(), "nobody in, nobody drives");
        assert_eq!(serde_json::to_value(&r).unwrap()["last"]["type"], "showMessage");
    }
}
