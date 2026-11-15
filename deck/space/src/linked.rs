//! The linked reader (#527, agents/research/SCENE-EVENTS.md §6): a reader page linked to a live session drives the decks'
//! story, one reader at a time – the newest claim wins, the relay's rule (relay session.rs, `reader.drive`). Here, once for
//! every client of the core (RULES A6): the link a conductor offers (`reader_link` – the deck editor and the conductor
//! page give the same text), a reader's place in the story (`Steer`, driven by relay.rs `Client::drive`), where its story
//! stands for a vote's result (`Place`, #552) and the conductor's view of the linked readers (`Readers`, relay.rs
//! `Client::readers`).
use crate::relay::{encode, parse_relay, relay_param, HOSTED_RELAY};
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
    // #535: with another relay goes its QUIC door as the conductor's URL names it (`?quic=`, `&quichash=`; relay.rs
    // `parse_quic`) – the reader page reads both (viewer live-deck.ts `relayOf`); `quic=off` and no `quic`: none
    let door = |url: &str| match relay_param(url, "quic") {
        Some(off) if off == "off" => String::new(),
        Some(door) => format!("&quic={}{}", encode(&door), relay_param(url, "quichash").filter(|h| !h.is_empty()).map(|h| format!("&quichash={}", encode(&h))).unwrap_or_default()),
        None => String::new(),
    };
    let relay = relay
        .and_then(|r| parse_relay(r).ok().map(|(b, _)| (b, r)))
        .filter(|(b, _)| !b.eq_ignore_ascii_case(&hosted))
        .map(|(b, r)| format!("&relay={}{}", encode(&b), door(r)))
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

/// #552 (SHOW.md §15 3, LIVE.md B.1): where a reader's story stands for a vote's result, read from the events it fires (every
/// one, sent or not): the `addChoice` ids since the last `showChoices` are the options it offers (one hidden by its "Only
/// if" never fires), a `showChoices` is where it stands until the next event fires or its runner says the choices closed.
#[derive(Debug, Clone, Default)]
pub struct Place {
    /// the options fired since the last `showChoices` (an option without an id: '')
    offered: Vec<String>,
    /// the `showChoices` it stands at ('' without an id) and the options it offers there
    at: Option<(String, Vec<String>)>,
}

/// Why a vote's result is not this reader's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ignored {
    /// it does not stand at that choice
    Elsewhere,
    /// it stands there, but the voted option is not one it offers (hidden by its "Only if", or without an id)
    NotOffered,
}

/// A story question's result as the reader's page hears it (relay.rs `Out::Choice`): pick the option at `pick` among the
/// ones it offers – as a tap would –, or it is ignored and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    /// the `showChoices`' id and the voted option's `addChoice` id ('' = none)
    pub event: String,
    pub option: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<Ignored>,
}

impl Place {
    /// an event the reader fired (the VNEvent: `type`, `id`)
    pub fn fired(&mut self, event: &Loose) {
        let id = event.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        self.at = None;
        match event.get("type").and_then(|t| t.as_str()) {
            Some("addChoice") => self.offered.push(id),
            Some("showChoices") => self.at = Some((id, std::mem::take(&mut self.offered))),
            _ => {}
        }
    }

    /// its runner says the choices closed (picked, or none was shown): it stands at none
    pub fn closed(&mut self) {
        self.at = None;
    }

    /// The result of the vote that asked `story`: option `option` (its place in the vote). A pick is taken: the reader
    /// stands at no choice after it.
    pub fn result(&mut self, story: &protocol::VoteStory, option: i64) -> Choice {
        let voted = usize::try_from(option).ok().and_then(|o| story.options.get(o)).cloned().unwrap_or_default();
        let mut choice = Choice { event: story.event.clone(), option: voted.clone(), pick: None, ignored: None };
        match &self.at {
            Some((event, offered)) if !event.is_empty() && *event == story.event => match offered.iter().position(|o| !voted.is_empty() && *o == voted) {
                Some(i) => {
                    choice.pick = Some(i);
                    self.at = None;
                }
                None => choice.ignored = Some(Ignored::NotOffered),
            },
            _ => choice.ignored = Some(Ignored::Elsewhere),
        }
        choice
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
        // #535: the relay's QUIC door goes with it, as the conductor's URL names it – the door, its pinned hash, an empty one
        // (the relay's own); `off` and none: none; the hosted relay's link stays as it is
        assert_eq!(link(Some("ws://192.168.1.116:7070/live/A7K2?quic=192.168.1.116:7071")), format!("{today}&relay=ws%3A%2F%2F192.168.1.116%3A7070&quic=192.168.1.116%3A7071"));
        assert_eq!(
            link(Some("ws://192.168.1.116:7070/live/A7K2?quic=192.168.1.116:7071&quichash=0a1B")),
            format!("{today}&relay=ws%3A%2F%2F192.168.1.116%3A7070&quic=192.168.1.116%3A7071&quichash=0a1B")
        );
        assert_eq!(link(Some("ws://192.168.1.116:7070/live/A7K2?quic=")), format!("{today}&relay=ws%3A%2F%2F192.168.1.116%3A7070&quic="));
        assert_eq!(link(Some("ws://192.168.1.116:7070/live/A7K2?quic=off&quichash=0a1b")), format!("{today}&relay=ws%3A%2F%2F192.168.1.116%3A7070"));
        assert_eq!(link(Some("wss://live.scutoons.com/live/A7K2?quic=live.scutoons.com:443")), today);
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

    fn fire(p: &mut Place, kind: &str, id: Option<&str>) {
        let mut e = json!({ "type": kind, "params": {} });
        if let Some(id) = id {
            e["id"] = json!(id);
        }
        p.fired(e.as_object().unwrap());
    }

    #[test]
    fn a_vote_s_result_is_the_reader_s_choice_only_where_it_stands() {
        let door = protocol::VoteStory { event: "ch3-door".into(), options: vec!["ch3-stay".into(), "ch3-go".into(), "".into()] };
        let pick = |c: &Choice| (c.pick, c.ignored);
        let mut p = Place::default();
        assert_eq!(pick(&p.result(&door, 0)), (None, Some(Ignored::Elsewhere)), "at no choice yet");
        // the options it offers: those that fired (one hidden by its "Only if" never does)
        fire(&mut p, "showMessage", Some("ch3-e1"));
        fire(&mut p, "addChoice", Some("ch3-stay"));
        fire(&mut p, "addChoice", Some("ch3-go"));
        fire(&mut p, "showChoices", Some("ch3-door"));
        let c = p.result(&door, 1);
        assert_eq!(c, Choice { event: "ch3-door".into(), option: "ch3-go".into(), pick: Some(1), ignored: None });
        assert_eq!(serde_json::to_value(&c).unwrap(), json!({ "event": "ch3-door", "option": "ch3-go", "pick": 1 }));
        assert_eq!(pick(&p.result(&door, 1)), (None, Some(Ignored::Elsewhere)), "a pick is taken: it stands at none after it");
        // hidden by its "Only if": stay never fired – go is the first it offers
        fire(&mut p, "addChoice", Some("ch3-go"));
        fire(&mut p, "showChoices", Some("ch3-door"));
        assert_eq!(pick(&p.result(&door, 0)), (None, Some(Ignored::NotOffered)), "the voted option is hidden here");
        assert_eq!(serde_json::to_value(p.result(&door, 2)).unwrap(), json!({ "event": "ch3-door", "option": "", "ignored": "not_offered" }), "an option without an id");
        assert_eq!(pick(&p.result(&door, 9)), (None, Some(Ignored::NotOffered)), "out of range");
        assert_eq!(pick(&p.result(&door, 1)), (Some(0), None), "its place among what it offers");
        // another choice; an event after the choice; the runner's word that the choices closed
        for (moved, why) in [(Some("label"), "an event after the choice"), (None, "the choices closed")] {
            fire(&mut p, "addChoice", Some("ch3-go"));
            fire(&mut p, "showChoices", Some("ch3-door"));
            match moved {
                Some(kind) => fire(&mut p, kind, Some("go")),
                None => p.closed(),
            }
            assert_eq!(pick(&p.result(&door, 1)), (None, Some(Ignored::Elsewhere)), "{why}");
        }
        fire(&mut p, "addChoice", Some("ch3-go"));
        fire(&mut p, "showChoices", Some("ch4-gate"));
        assert_eq!(pick(&p.result(&door, 1)), (None, Some(Ignored::Elsewhere)), "another choice");
        fire(&mut p, "addChoice", Some("ch3-go"));
        fire(&mut p, "showChoices", None);
        assert_eq!(pick(&p.result(&protocol::VoteStory { event: "".into(), options: vec!["ch3-go".into()] }, 0)).1, Some(Ignored::Elsewhere), "a choice without an id is nobody's");
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
