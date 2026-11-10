//! A deck's flash side, step by step (#475; #91 item 5, #471, agents/research/SHOW.md §4b): for a deck that cannot wait
//! on a thread (the browser's frame loop). The decisions are [`ready::Deck`]'s – taken or refused, the show this outlet
//! plays, what it lacks, the words. This is the order of the answers, sans IO:
//! - **the show** at the version the flash pins: the platform is asked for it once (`ask`: `<api>/shows/<id>?v=<n>`,
//!   with the deck's token) and hands the API's answer back (`document`); it is kept while the deck runs (a version
//!   never changes);
//! - **each piece** its outlet's cells name: what the deck carries (`carried`: a built-in scene), a core rig, else what
//!   it fetched (deck-space `fetched.rs`); the check waits until every answer is here;
//! - **the answer**: the `flash.report`s to send (`reports`: also when the deck joins – what it is, what it runs).
//!
//! The native deck does the same on its own thread, blocking (deck-native live/flash.rs).
use crate::ready::{self, Deck};
use crate::{Pinned, Show};
use deck_space::fetched::Fetched;
use deck_space::piece::{available, Found};
use deck_space::rig::address::Place;
use protocol::{Flash, FlashReport};
use std::collections::HashMap;

/// the shows kept: a deck plays from a handful (no growth without bound, RULES A10)
const KEEP: usize = 16;

/// A flash taken, until its report went out.
#[derive(Debug, Clone)]
struct Checking {
    show: Pinned,
    /// the show's document or why it is not here; None = asked of the platform
    doc: Option<Result<Show, String>>,
    /// its pieces were asked for
    asked: bool,
}

#[derive(Debug, Clone)]
pub struct Flashed {
    pub deck: Deck,
    outlet: String,
    /// the shows fetched, by [`ready::kept_as`]
    shows: HashMap<String, String>,
    checking: Option<Checking>,
    /// what the platform fetches next: the kept name and the URL
    ask: Option<(String, String)>,
    /// the reports to send
    out: Vec<FlashReport>,
}

impl Flashed {
    /// the flash side of `outlet` on a deck of this kind (native, browser, phone …)
    pub fn new(outlet: &str, device: &str) -> Flashed {
        let mut deck = Deck::new(outlet);
        deck.set_device(device);
        Flashed { deck, outlet: outlet.to_owned(), shows: HashMap::new(), checking: None, ask: None, out: Vec::new() }
    }

    /// the relay's welcome: this deck's client id (a flash's lists name it so); the deck says what it is
    pub fn joined(&mut self, id: &str) {
        self.deck.set_id(id);
        let r = self.deck.about();
        self.out.push(r);
    }

    /// A flash arrives. `api`: the API's base; `no_token`: why the deck cannot ask it ("" = it can).
    pub fn flash(&mut self, flash: &Flash, api: &str, no_token: &str, fetched: &mut Fetched) {
        let show = match self.deck.take(flash) {
            Err(refused) => return self.out.push(*refused),
            Ok(show) => show,
        };
        // a piece that failed before is asked for again
        fetched.again();
        let key = ready::kept_as(&show);
        let doc = match self.shows.get(&key) {
            Some(text) => Some(ready::read(text)),
            None if !no_token.is_empty() => Some(Err(no_token.to_owned())),
            None => {
                self.ask = Some((key, format!("{}/shows/{}?v={}", api.trim_end_matches('/'), show.id, show.version)));
                None
            }
        };
        self.checking = Some(Checking { show, doc, asked: false });
    }

    /// what the platform fetches with the deck's token: the name the show is kept as (`document`'s key) and the URL
    pub fn ask(&mut self) -> Option<(String, String)> {
        self.ask.take()
    }

    /// The platform's answer for the show kept as `key`: the API's status and body, or why the fetch failed. false = it
    /// was not asked for.
    pub fn document(&mut self, key: &str, answer: Result<(u16, &str), String>) -> bool {
        let Some(c) = self.checking.as_mut().filter(|c| c.doc.is_none() && ready::kept_as(&c.show) == key) else { return false };
        let doc = match answer {
            Ok((200, body)) => ready::read(body).inspect(|_| {
                if self.shows.len() >= KEEP {
                    self.shows.clear();
                }
                self.shows.insert(key.to_owned(), body.to_owned());
            }),
            Ok((status, body)) => Err(refusal(status, body)),
            Err(why) => Err(why),
        };
        c.doc = Some(doc);
        true
    }

    /// One turn (a frame): a flash whose show is here asks for its pieces once; when every answer is here, the report.
    /// `carried`: what the deck has of a piece without asking (a built-in scene).
    pub fn turn(&mut self, fetched: &mut Fetched, carried: impl Fn(&Place) -> Option<Found>) {
        let Some(c) = self.checking.as_mut() else { return };
        let Some(doc) = c.doc.as_ref() else { return };
        if !c.asked {
            c.asked = true;
            for need in doc.as_ref().map(|d| ready::needs(d, &self.outlet)).unwrap_or_default() {
                let _ = available(&need.piece, |p| carried(p).or_else(|| fetched.find(p)));
            }
        }
        if fetched.waiting() {
            return;
        }
        let Some(Checking { show, doc: Some(doc), .. }) = self.checking.take() else { return };
        let fetched = &*fetched;
        let report = self.deck.check(&show, doc, |piece| available(piece, |p| carried(p).or_else(|| fetched.have(p))));
        self.out.push(report);
    }

    /// the reports to send, oldest first
    pub fn reports(&mut self) -> Vec<FlashReport> {
        std::mem::take(&mut self.out)
    }

    /// "please don't flash" and the notice after a flash replaced the deck's own version, for the HUD
    pub fn notice(&self) -> Vec<String> {
        self.deck.keep_line().into_iter().chain(self.deck.notice()).collect()
    }
}

/// the API refused: its message when it has one
fn refusal(status: u16, body: &str) -> String {
    let said = serde_json::from_str::<serde_json::Value>(body).ok().and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned));
    match said {
        Some(m) => format!("HTTP {status}: {m}"),
        None => format!("HTTP {status}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_space::scene::builtin;

    /// a show with the outlet `left`: a built-in scene, then a character whose rig the deck does not carry
    const SHOW: &str = r#"{ "id": "top", "version": 3, "outlets": [{ "id": "left" }],
        "rows": [{ "id": "a", "cells": { "left": { "mode": "play", "piece": "scene:cathedral" } } },
                 { "id": "b", "cells": { "left": { "mode": "play", "piece": "character:boy" } } }] }"#;
    /// scene-content as the server writes it: boy has no rig
    const CONTENT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content.json"));
    const API: &str = "https://api.test/api/";

    fn flash(id: &str) -> Flash {
        let top = protocol::Pinned { id: "top".into(), version: 3 };
        Flash { root: top.clone(), shows: vec![top], lock: None, hops: 0, here: None, id: Some(id.into()), by: None, skip: None, force: None }
    }

    fn carried(p: &Place) -> Option<Found> {
        builtin(&p.id).filter(|_| p.kind == "scene").map(|t| Found::Scene(t.to_string()))
    }

    #[test]
    fn a_flash_is_answered_when_the_show_and_its_pieces_are_here() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.joined("d1");
        assert_eq!(f.reports().len(), 1, "what the deck is, when it joins");
        f.flash(&flash("one"), API, "", &mut fetched);
        assert_eq!(f.ask(), Some(("shows/top@3.json".to_string(), "https://api.test/api/shows/top?v=3".to_string())));
        f.turn(&mut fetched, carried);
        assert!(f.reports().is_empty(), "the show is on its way");
        assert!(!f.document("shows/other@1.json", Ok((200, SHOW))), "not asked for");
        // the API's answer as it is (`show`); then the piece the deck does not carry is asked for
        assert!(f.document("shows/top@3.json", Ok((200, &format!(r#"{{ "status": "success", "show": {SHOW} }}"#)))));
        f.turn(&mut fetched, carried);
        let boy = Place::new("character", "boy");
        assert_eq!(fetched.asks().len(), 1);
        f.turn(&mut fetched, carried);
        assert!(f.reports().is_empty(), "the piece is on its way");
        fetched.give(&boy, Ok(CONTENT));
        f.turn(&mut fetched, carried);
        let r = f.reports();
        assert_eq!((r.len(), r[0].ready, r[0].device.as_deref()), (1, Some(false), Some("browser")));
        assert_eq!(r[0].lacks.as_deref(), Some(&["row b: character:boy has no published rig".to_string()][..]));
        // the same show flashed again: kept, nothing is asked of the platform; the piece is asked for again
        f.flash(&flash("two"), API, "", &mut fetched);
        assert_eq!(f.ask(), None);
        f.turn(&mut fetched, carried);
        assert_eq!(fetched.asks().len(), 1);
    }

    #[test]
    fn a_show_that_is_not_here_is_what_the_deck_lacks() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.flash(&flash("one"), API, "", &mut fetched);
        assert!(f.document("shows/top@3.json", Ok((403, r#"{ "status": "error", "message": "The role live is required." }"#))));
        f.turn(&mut fetched, carried);
        assert_eq!(f.reports()[0].lacks.as_deref(), Some(&["top@3: the show is not here (HTTP 403: The role live is required.)".to_string()][..]));
        // a deck without a token says why and asks nothing
        let mut g = Flashed::new("left", "browser");
        g.flash(&flash("one"), API, "the deck is not paired yet", &mut fetched);
        assert_eq!(g.ask(), None);
        g.turn(&mut fetched, carried);
        assert_eq!(g.reports()[0].lacks.as_deref(), Some(&["top@3: the show is not here (the deck is not paired yet)".to_string()][..]));
    }
}
