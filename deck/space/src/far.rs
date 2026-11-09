//! Events between linked decks (#442, research/MODULE-WIRING.md §3, EVENTS-VS-PARAMS.md §5 "another deck is a world
//! node with a stated way of arriving"): the rules of both sides, platform-free – the link thread of a deck drives them.
//!
//! - **Asking.** A deck whose rig has far world nodes (world.rs `Source::Far`) sends their absolute addresses as
//!   `deck.listen` to the group `decks`, on each welcome and every `LISTEN_MS` (relay.rs `Client::set_listen`). A
//!   subscription not refreshed for `LAPSE_MS` lapses (`Subscriptions`), so a deck that left stops being served.
//! - **Offering.** The deck an address names answers: its `deck:` place is this deck's name (deck.toml `name`), its holder
//!   place (`character:` / `item:`) is the rig it plays, the leaf names one of its event outputs (`offers`). The scene
//!   place is not checked until scenes hold rigs. What else an address names gets no answer, so the asking node stays
//!   silent (wiring.rs lists every far node as playing only while the deck it names is in the session).
//! - **Sending.** Every event output of the engine writes deck-events events into its outbox ring (deck-engine evhost
//!   "out to the world"); the link drains them (`FarTable::drain`) and sends what a subscriber asked for as `deck.events`
//!   to that client alone: the name, the props by key, a note's length in seconds (`leaving`).
//! - **Arriving.** `deck.events` from any deck: each event goes into the ring of every far world node cabled under that
//!   address whose `listen` takes the name (`arrived`, `FarTable::take`), as a deck-events event with this engine's
//!   names, at `Time::Now` – it plays at frame 0 of the next block (one late is counted, deck-engine evhost). Lining
//!   far events up on relay time is a later slice.
//!
//! The rings are the engine's (rtrb); here they are closures, so this crate names no ring type.
use crate::relay::{Client, Out};
use crate::rig::address::{Address, Place};
use deck_events::{Event, Names, Time};
use protocol::DeckEvent;
use std::collections::HashMap;

/// a deck with far world nodes asks again this often (ms)
pub const LISTEN_MS: f64 = 10_000.0;
/// a subscription not asked for again within this lapses (ms)
pub const LAPSE_MS: f64 = 30_000.0;

/// one far world node's cable: where its events go in
pub struct FarInlet {
    /// the absolute address the node names (as written)
    pub address: String,
    /// the names the node lets through (world.rs `listen`)
    pub listen: Vec<String>,
    /// into the engine's ring (false: full – the event is dropped)
    pub push: Box<dyn FnMut(Event) -> bool + Send>,
}

/// one event output of this deck's engine: where its events come out
pub struct FarOutlet {
    /// the compiled slot `<node>.<port>` (a flattened rig's `<track>_<node>.<port>`)
    pub slot: String,
    /// the engine's ring, drained
    pub pop: Box<dyn FnMut() -> Option<Event> + Send>,
}

/// What a deck's link holds for far events: who this deck is, its engine's names, where far events go in and where its own
/// come out.
pub struct FarTable {
    /// this deck's name (deck.toml `name`): the place `deck:<name>`; none = it answers no listen
    pub name: Option<String>,
    /// the rig it plays (the played scene's name: `character.catpope`)
    pub subject: String,
    pub names: Names,
    pub inlets: Vec<FarInlet>,
    pub outlets: Vec<FarOutlet>,
    /// the engine's tempo (a length in beats becomes seconds on it; none = 120 bpm)
    pub tempo: Option<std::sync::Arc<deck_core::tempo::Shared>>,
}

impl FarTable {
    /// the addresses this deck asks for, each once, in order
    pub fn addresses(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for i in &self.inlets {
            if !out.contains(&i.address) {
                out.push(i.address.clone());
            }
        }
        out
    }

    /// the slots this deck can offer
    pub fn slots(&self) -> Vec<String> {
        self.outlets.iter().map(|o| o.slot.clone()).collect()
    }

    /// the (address, slot) pairs of `addresses` this deck answers (`offers`)
    pub fn offers(&self, addresses: &[String]) -> Vec<(String, String)> {
        offers(self.name.as_deref(), &self.subject, &self.slots(), addresses)
    }

    /// Events that arrived for this deck: each into every inlet of its address whose filter takes the name; the number
    /// pushed.
    pub fn take(&mut self, events: &[DeckEvent]) -> usize {
        let mut n = 0;
        for e in events {
            let Some(ev) = arrived(e, &self.names) else { continue };
            for inlet in self.inlets.iter_mut().filter(|i| i.address == e.address && crate::world::shares_a_name(std::slice::from_ref(&e.name), &i.listen)) {
                if (inlet.push)(ev.clone()) {
                    n += 1;
                }
            }
        }
        n
    }

    /// Drain every outlet: what a subscriber asked for goes to `out` as (client, event); the rest is dropped. A length
    /// in beats becomes seconds on the engine's tempo.
    pub fn drain(&mut self, subs: &Subscriptions, out: &mut Vec<(String, DeckEvent)>) {
        let bpm = self.tempo.as_ref().and_then(|t| t.map()).map_or(120.0, |(m, _)| m.bpm);
        for o in &mut self.outlets {
            while let Some(e) = (o.pop)() {
                for (client, address) in subs.wanting(&o.slot) {
                    out.push((client.to_string(), leaving(&e, &self.names, address, bpm)));
                }
            }
        }
    }
}

/// Which of `addresses` a deck named `name`, playing `subject`, with the event output `slots`, answers: (address, slot).
pub fn offers(name: Option<&str>, subject: &str, slots: &[String], addresses: &[String]) -> Vec<(String, String)> {
    let Some(name) = name else { return Vec::new() };
    let own = Place::of_subject(subject);
    addresses
        .iter()
        .filter_map(|text| {
            let a = Address::parse(text).ok()?;
            let deck = a.places.iter().find(|p| p.kind == "deck")?;
            if deck.id != name {
                return None;
            }
            let holder = a.places.iter().find(|p| p.kind == "character" || p.kind == "item")?;
            if Some(holder) != own.as_ref() {
                return None;
            }
            let slot = a.leaf.slot()?;
            slots.contains(&slot).then(|| (text.clone(), slot))
        })
        .collect()
}

/// a deck-events event as the wire carries it: its name and props by key, a length in seconds (beats on `bpm`)
pub fn leaving(e: &Event, names: &Names, address: &str, bpm: f64) -> DeckEvent {
    let dur_s = match e.dur {
        Some(Time::Seconds(s)) => Some(s),
        Some(Time::Beats(b)) if bpm > 0.0 => Some(b * 60.0 / bpm),
        _ => None,
    };
    DeckEvent {
        address: address.to_string(),
        name: names.name(e.name).to_string(),
        props: e.props.iter().map(|(k, v)| (names.name(k).to_string(), f64::from(v))).collect(),
        dur_s,
    }
}

/// a wire event as this engine's deck-events event, at `Time::Now`; none when the engine never interned the name
pub fn arrived(e: &DeckEvent, names: &Names) -> Option<Event> {
    let name = names.get(&e.name)?;
    let mut ev = Event::new(name, Time::Now);
    for (k, v) in &e.props {
        if let Some(key) = names.get(k) {
            ev.props.set(key, *v as f32);
        }
    }
    if let Some(d) = e.dur_s {
        ev.dur = Some(Time::Seconds(d));
    }
    ev.source = Some(format!("far:{}", e.address));
    Some(ev)
}

/// A deck's side of far events on its link thread, platform-free: the table once the frame handed it over, who asked
/// this deck for what, the per-poll drain. deck-native's link.rs only calls `attach`, `poll` and `hear`.
#[derive(Default)]
pub struct Side {
    table: Option<FarTable>,
    subs: Subscriptions,
    /// scratch: (client, event) per poll
    sending: Vec<(String, DeckEvent)>,
}

impl Side {
    /// the engine's side arrived from the frame: ask for its far addresses, forget who asked the old engine
    pub fn attach(&mut self, table: FarTable, c: &mut Client, now: f64) {
        c.set_listen(now, table.addresses());
        self.table = Some(table);
        self.subs = Subscriptions::default();
    }

    /// each poll: this deck's event outputs to their subscribers (one message per client, in arrival order), lapsed
    /// subscriptions dropped
    pub fn poll(&mut self, c: &mut Client, now: f64) {
        let Some(table) = self.table.as_mut() else { return };
        self.subs.expire(now);
        self.sending.clear();
        table.drain(&self.subs, &mut self.sending);
        while let Some((client, _)) = self.sending.first() {
            let client = client.clone();
            let events: Vec<DeckEvent> = self.sending.iter().filter(|(cl, _)| *cl == client).map(|(_, e)| e.clone()).collect();
            self.sending.retain(|(cl, _)| *cl != client);
            c.events(&client, events);
        }
    }

    /// what the client says: another deck's ask (what names this deck's outputs is served from now on) or events for
    /// this deck's far world nodes (into their rings). True when it was one of these.
    pub fn hear(&mut self, out: &Out, now: f64) -> bool {
        match out {
            Out::Listen { client, addresses } => {
                if let Some(t) = &self.table {
                    self.subs.listen(client, t.offers(addresses), now);
                }
                true
            }
            Out::Events { events, .. } => {
                if let Some(t) = self.table.as_mut() {
                    t.take(events);
                }
                true
            }
            _ => false,
        }
    }

    /// who asks this deck now (the HUD, tests)
    pub fn asked_by(&self) -> usize {
        self.subs.by_client.len()
    }
}

/// Who asked this deck for which of its outputs: client → ((address, slot) pairs, when it last asked).
#[derive(Default)]
pub struct Subscriptions {
    by_client: HashMap<String, (Vec<(String, String)>, f64)>,
}

impl Subscriptions {
    /// `client` asked (again) for `offered` – what `offers` matched of its listen; nothing matched = it is gone
    pub fn listen(&mut self, client: &str, offered: Vec<(String, String)>, now: f64) {
        if offered.is_empty() {
            self.by_client.remove(client);
        } else {
            self.by_client.insert(client.to_string(), (offered, now));
        }
    }

    /// drop what was not asked for again within `LAPSE_MS`
    pub fn expire(&mut self, now: f64) {
        self.by_client.retain(|_, (_, at)| now - *at < LAPSE_MS);
    }

    pub fn is_empty(&self) -> bool {
        self.by_client.is_empty()
    }

    /// the (client, address) pairs that want `slot`
    pub fn wanting<'a>(&'a self, slot: &'a str) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.by_client.iter().flat_map(move |(client, (pairs, _))| pairs.iter().filter(move |(_, s)| s == slot).map(move |(a, _)| (client.as_str(), a.as_str())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const SEQ: &str = "/deck:left/scene:nave/character:catpope/analog:seq.notes";

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn a_deck_offers_what_names_it_its_rig_and_an_output_it_has() {
        let slots = s(&["analog_seq.notes", "voice_melody.cv"]);
        let asked = s(&[SEQ, "/deck:right/scene:nave/character:catpope/analog:seq.notes", "/deck:left/scene:nave/character:boy/analog:seq.notes", "/deck:left/scene:nave/character:catpope/analog:seq.nope", "/deck:left/scene:nave/character:catpope/voice:@gain", "character:catpope/analog:seq.notes", "nonsense//"]);
        assert_eq!(offers(Some("left"), "character.catpope", &slots, &asked), [(SEQ.to_string(), "analog_seq.notes".to_string())]);
        assert_eq!(offers(Some("left"), "character.catpope.pope", &slots, &asked).len(), 1, "a persona's rig is the character's place");
        assert!(offers(None, "character.catpope", &slots, &asked).is_empty(), "a deck without a name answers nobody");
        assert!(offers(Some("left"), "item.incense_burner", &slots, &asked).is_empty());
    }

    #[test]
    fn an_event_crosses_the_wire_and_comes_back_with_this_engines_names() {
        let mut names = Names::new();
        let (note, pitch, vel) = (names.event("music.note").unwrap(), names.key("pitch").unwrap(), names.key("vel").unwrap());
        let e = Event::beats(note, 3.0).lasting(Time::Beats(0.5)).with(pitch, 440.0).with(vel, 0.8);
        let w = leaving(&e, &names, SEQ, 120.0);
        assert_eq!((w.address.as_str(), w.name.as_str(), w.dur_s), (SEQ, "music.note", Some(0.25)));
        assert_eq!((w.props["pitch"], w.props["vel"] as f32), (440.0, 0.8f32), "props as the f32 they were");
        let back = arrived(&w, &names).unwrap();
        assert_eq!((back.name, back.start, back.dur), (note, Time::Now, Some(Time::Seconds(0.25))));
        assert_eq!((back.props.get(pitch), back.props.get(vel)), (Some(440.0), Some(0.8)));
        assert_eq!(back.source.as_deref(), Some(&format!("far:{SEQ}")[..]));
        let unknown = DeckEvent { address: SEQ.into(), name: "story.cue".into(), props: HashMap::new(), dur_s: None };
        assert!(arrived(&unknown, &names).is_none(), "a name this engine never interned reaches nobody");
    }

    #[test]
    fn subscriptions_lapse_and_a_table_routes_both_ways() {
        let mut subs = Subscriptions::default();
        subs.listen("c7", vec![(SEQ.to_string(), "analog_seq.notes".to_string())], 0.0);
        subs.listen("c8", vec![("x".to_string(), "voice_melody.cv".to_string())], 0.0);
        assert_eq!(subs.wanting("analog_seq.notes").collect::<Vec<_>>(), [("c7", SEQ)]);
        subs.expire(LAPSE_MS - 1.0);
        assert!(!subs.is_empty());
        subs.listen("c8", vec![("x".to_string(), "voice_melody.cv".to_string())], LAPSE_MS);
        subs.expire(LAPSE_MS + 1.0);
        assert!(subs.wanting("analog_seq.notes").next().is_none() && subs.wanting("voice_melody.cv").next().is_some(), "c7 lapsed, c8 asked again");
        subs.listen("c8", vec![], LAPSE_MS);
        assert!(subs.is_empty(), "a listen that matches nothing ends it");

        // a table: an outlet's note goes to the subscriber; an arriving event lands in the inlets of its address that take the name
        let mut names = Names::new();
        let note = names.event("music.note").unwrap();
        names.event("story.cue").unwrap();
        let queue = Arc::new(Mutex::new(vec![Event::now(note)]));
        let got = Arc::new(Mutex::new(Vec::<Event>::new()));
        let (q, g, g2) = (queue.clone(), got.clone(), got.clone());
        let mut table = FarTable {
            name: Some("left".into()),
            subject: "character.catpope".into(),
            names,
            inlets: vec![
                FarInlet { address: SEQ.into(), listen: s(&["music.note"]), push: Box::new(move |e| { g.lock().unwrap().push(e); true }) },
                FarInlet { address: SEQ.into(), listen: s(&["story.cue"]), push: Box::new(move |e| { g2.lock().unwrap().push(e); true }) },
            ],
            outlets: vec![FarOutlet { slot: "analog_seq.notes".into(), pop: Box::new(move || q.lock().unwrap().pop()) }],
            tempo: None,
        };
        assert_eq!(table.addresses(), [SEQ]);
        let mut subs = Subscriptions::default();
        subs.listen("c7", table.offers(&s(&[SEQ])), 0.0);
        let mut out = Vec::new();
        table.drain(&subs, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].0.as_str(), out[0].1.name.as_str()), ("c7", "music.note"));
        assert_eq!(table.take(&out.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>()), 1, "the note inlet takes it, the cue inlet does not");
        assert_eq!(got.lock().unwrap().len(), 1);
    }

    /// the link's side: the table attached asks once joined, another deck's ask is served on the next poll, events for
    /// this deck land in their ring, anything else is not its business
    #[test]
    fn the_side_asks_serves_and_takes() {
        use crate::relay::{Options, DECKS};
        use protocol::Role;
        let mut names = Names::new();
        let note = names.event("music.note").unwrap();
        let queue = Arc::new(Mutex::new(Vec::<Event>::new()));
        let got = Arc::new(Mutex::new(Vec::<Event>::new()));
        let (q, g) = (queue.clone(), got.clone());
        let table = FarTable {
            name: Some("left".into()),
            subject: "character.catpope".into(),
            names,
            inlets: vec![FarInlet { address: "/deck:right/scene:x/character:boy/voice:seq.notes".into(), listen: s(&["music.note"]), push: Box::new(move |e| { g.lock().unwrap().push(e); true }) }],
            outlets: vec![FarOutlet { slot: "analog_seq.notes".into(), pop: Box::new(move || q.lock().unwrap().pop()) }],
            tempo: None,
        };
        let mut c = Client::new(Options::new("ws://h:7700/live/CATH", Role::Musician, Some(DECKS))).unwrap();
        c.start(0.0);
        c.opened(0.0);
        c.text(0.0, r#"{"v":1,"seq":1,"from":"relay","kind":"welcome","p":{"client":"c3","snapshot":{},"now":0}}"#);
        c.take();
        let mut side = Side::default();
        side.attach(table, &mut c, 0.0);
        let kinds = |outs: Vec<Out>| -> Vec<String> { outs.into_iter().filter_map(|o| if let Out::Send(t) = o { serde_json::from_str::<serde_json::Value>(&t).ok().map(|v| v["kind"].as_str().unwrap_or("").to_string()) } else { None }).collect() };
        assert_eq!(kinds(c.take()), ["deck.listen"]);
        assert!(side.hear(&Out::Listen { client: "c7".into(), addresses: s(&[SEQ]) }, 1.0));
        assert_eq!(side.asked_by(), 1);
        queue.lock().unwrap().push(Event::now(note));
        side.poll(&mut c, 2.0);
        assert_eq!(kinds(c.take()), ["deck.events"], "the subscriber gets the note");
        assert!(side.hear(&Out::Events { client: "c9".into(), events: vec![DeckEvent { address: "/deck:right/scene:x/character:boy/voice:seq.notes".into(), name: "music.note".into(), props: HashMap::new(), dur_s: None }] }, 3.0));
        assert_eq!(got.lock().unwrap().len(), 1);
        assert!(!side.hear(&Out::Close {}, 3.0));
        side.poll(&mut c, 3.0 + LAPSE_MS);
        assert_eq!(side.asked_by(), 0, "c7 never asked again");
    }
}
