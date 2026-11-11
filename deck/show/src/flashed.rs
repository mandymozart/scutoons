//! A deck's flash side, step by step (#475; #91 item 5, #471, agents/research/SHOW.md §4b): for a deck that cannot wait
//! on a thread (the browser's frame loop). The decisions are [`ready::Deck`]'s – taken or refused, the show this outlet
//! plays, what it lacks, the words. This is the order of the answers, sans IO:
//! - **what the outlet plays**: the flash says it (`Flash.needs`, #475: the conductor sends a deck the part of the show
//!   that matters to it) – then nothing is read. Only a flash that does not say it needs **the show** at the pinned
//!   version: the platform is asked for it once (`ask`: `<api>/shows/<id>?v=<n>`, with the deck's token) and hands the
//!   API's answer back (`document`); it is kept while the deck runs (a version never changes);
//! - **each piece** its outlet's cells name: what the deck carries (`carried`: a built-in scene), a core rig, else what
//!   it fetched (deck-space `fetched.rs`); the check waits until every answer is here;
//! - **the answer**: the `flash.report`s to send (`reports`: also when the deck joins – what it is, what it runs).
//!
//! - **the person at the deck** (#481, `dialog.rs`): a flash that is only suggested waits for them – accept, reject, or
//!   no answer for `SUGGESTION_SECS`, which accepts – when they may reject (`set_account`: the role `live` in the
//!   deck's live proof; a guest and a member may
//!   not: they are flashed and read a toast). After a flash that replaced the deck's own version they get the notice
//!   with the way back; everyone else the toast. Who may reject may also say "please don't flash" at the deck
//!   (`open_keep`: a switch; owner 2026-11-11: no reason). `shown` is what `face.rs` draws, `press` / `key` what it
//!   hands back.
//!
//! The native deck runs the same on its own thread, where a fetch may block (deck-native live/flash.rs).
use crate::dialog::{self, Act, Key, Row, Shown};
use crate::ready::{self, Deck, SUGGESTION_SECS};
use crate::{Pinned, Show};
use deck_space::fetched::Fetched;
use deck_space::piece::{available, Found};
use deck_space::rig::address::Place;
use protocol::{Flash, FlashReport};
use std::collections::HashMap;

/// the shows kept: a deck plays from a handful (no growth without bound, RULES A10)
const KEEP: usize = 16;

/// how long a toast stays, seconds
const TOAST_SECS: f32 = 5.0;

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
    /// #481: the person at the deck may reject a suggested flash, and the language they read
    may_reject: bool,
    locale: String,
    /// a suggested flash waiting for them, and since when (the deck's clock, s)
    pending: Option<(Flash, Option<f32>)>,
    /// the flash they rejected last (a rejoin hands it over again: the same answer)
    rejected: Option<String>,
    /// the toast and until when it shows
    toast: Option<(Shown, f32)>,
    /// "please don't flash" while it is being set: on or off
    editing: Option<bool>,
    /// the row or button the keys are on
    focus: usize,
    /// the flash taken last was forced onto this deck
    forced: bool,
    now: f32,
    /// the API's base and why the deck cannot ask it, as the last flash came with them
    api: String,
    no_token: String,
}

impl Flashed {
    /// the flash side of `outlet` on a deck of this kind (native, browser, phone …)
    pub fn new(outlet: &str, device: &str) -> Flashed {
        let mut deck = Deck::new(outlet);
        deck.set_device(device);
        Flashed {
            deck,
            outlet: outlet.to_owned(),
            shows: HashMap::new(),
            checking: None,
            ask: None,
            out: Vec::new(),
            may_reject: false,
            locale: "en".into(),
            pending: None,
            rejected: None,
            toast: None,
            editing: None,
            focus: 0,
            forced: false,
            now: 0.0,
            api: String::new(),
            no_token: String::new(),
        }
    }

    /// #481: who is at the deck – the account roles its live proof states (deck-outlet `proof::account`; owner
    /// 2026-11-11: "Dann die live nachweis ob abgelehnt werden darf"): with `ready::MAY_REJECT` among them they may
    /// reject a suggested flash and ask not to be flashed. No proof, or a guest's: none.
    pub fn set_account(&mut self, roles: &[String]) {
        self.may_reject = roles.iter().any(|r| r == ready::MAY_REJECT);
    }

    /// the tests' short way to someone who may reject
    #[cfg(test)]
    pub(crate) fn set_may_reject(&mut self, yes: bool) {
        self.may_reject = yes;
    }

    /// the language of what the deck says to them (`de…` = German, else English)
    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_owned();
    }

    /// the relay's welcome: this deck's client id (a flash's lists name it so); the deck says what it is
    pub fn joined(&mut self, id: &str) {
        self.deck.set_id(id);
        let r = self.deck.about();
        self.out.push(r);
    }

    /// A flash arrives. `api`: the API's base; `no_token`: why the deck cannot ask it ("" = it can).
    pub fn flash(&mut self, flash: &Flash, api: &str, no_token: &str, fetched: &mut Fetched) {
        (self.api, self.no_token) = (api.to_owned(), no_token.to_owned());
        // a suggestion that was rejected here comes again (a rejoin): the same answer, quietly
        if flash.id.is_some() && flash.id == self.rejected {
            let report = self.deck.reject(flash);
            return self.out.push(report);
        }
        // #481: only suggested, and someone here may say no: it waits for them (`turn` counts, `press` / `key` answer)
        // (a deck that asks not to be flashed is not asked: it refuses by itself)
        if self.may_reject && self.deck.keep().is_none() && self.deck.suggested(flash) {
            self.focus = 0;
            self.pending = Some((flash.clone(), None));
            return;
        }
        self.take(flash, fetched);
    }

    fn take(&mut self, flash: &Flash, fetched: &mut Fetched) {
        let before = self.deck.runs().cloned();
        self.forced = self.deck.forced(flash);
        let show = match self.deck.take(flash) {
            Err(refused) => return self.out.push(*refused),
            Ok(show) => show,
        };
        // what the person reads: the notice when their own version was replaced and they may act on it, else a toast;
        // a flash that changed nothing (a rejoin hands the same one again) says nothing
        self.focus = 0;
        if before.as_ref() != Some(&show) {
            self.toast = (!self.notice_due()).then(|| (dialog::flashed(&self.locale, &show, flash.by.as_deref()), self.now + TOAST_SECS));
        }
        self.check(show, fetched);
    }

    /// the show to play is known: its document (unless the flash said what the outlet plays), then its pieces (`turn`)
    fn check(&mut self, show: Pinned, fetched: &mut Fetched) {
        // a piece that failed before is asked for again
        fetched.again();
        let key = ready::kept_as(&show);
        let doc = match self.shows.get(&key) {
            // the flash said what the outlet plays: no document
            _ if self.deck.told(&show).is_some() => None,
            Some(text) => Some(ready::read(text)),
            None if !self.no_token.is_empty() => Some(Err(self.no_token.clone())),
            None => {
                self.ask = Some((key, format!("{}/shows/{}?v={}", self.api.trim_end_matches('/'), show.id, show.version)));
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

    fn notice_due(&self) -> bool {
        self.may_reject && self.deck.unseen()
    }

    /// What the deck says to the person at it now (#481): the suggestion with its countdown, "please don't flash" while
    /// it is set, the notice, the toast – or nothing.
    pub fn shown(&self) -> Option<Shown> {
        if let Some((flash, since)) = &self.pending {
            let gone = since.map_or(0.0, |s| (self.now - s) / SUGGESTION_SECS);
            return Some(Shown::Dialog(dialog::suggestion(&self.locale, &ready::show_of(flash), flash.by.as_deref(), self.deck.runs(), gone, self.focus)));
        }
        if let Some(on) = self.editing {
            return Some(Shown::Dialog(dialog::keep(&self.locale, self.deck.runs(), on, self.focus)));
        }
        if let Some(kept) = self.deck.kept().filter(|_| self.notice_due()) {
            return Some(Shown::Dialog(dialog::notice(&self.locale, kept, self.forced, self.deck.may_switch(), self.focus)));
        }
        self.toast.as_ref().filter(|(_, until)| self.now < *until).map(|(toast, _)| toast.clone())
    }

    /// "Please don't flash" opens to be set (a key, a menu of the platform): only for who may reject, and not over
    /// another dialog. false = it did not open.
    pub fn open_keep(&mut self) -> bool {
        if !self.may_reject || matches!(self.shown(), Some(Shown::Dialog(_))) {
            return false;
        }
        self.editing = Some(self.deck.keep().is_some());
        self.focus = 0;
        true
    }

    /// Place `index` of what is shown was pressed (a tap, a click): a dialog's rows top down, then its buttons; a toast
    /// goes.
    pub fn press(&mut self, index: usize, fetched: &mut Fetched) {
        let d = match self.shown() {
            Some(Shown::Dialog(d)) => d,
            Some(Shown::Toast(..)) => return self.toast = None,
            None => return,
        };
        match d.rows.get(index) {
            Some(Row::Switch(..)) => {
                self.focus = index;
                if let Some(on) = self.editing.as_mut() {
                    *on = !*on;
                }
            }
            None => {
                if let Some((_, act)) = d.buttons.get(index - d.rows.len()) {
                    self.act(*act, fetched);
                }
            }
        }
    }

    /// A key of a deck without touch: up and down (Tab) go through the rows and buttons, left and right move between
    /// the buttons or flip a switch, Enter presses what the keys are on, Escape rejects a suggestion, leaves "please
    /// don't flash" as it was or closes a notice.
    pub fn key(&mut self, key: Key, fetched: &mut Fetched) {
        let Some(Shown::Dialog(d)) = self.shown() else { return };
        let (rows, buttons) = (d.rows.len(), d.buttons.len().max(1));
        let n = rows + buttons;
        match key {
            Key::Up => self.focus = (d.focus + n - 1) % n,
            Key::Down => self.focus = (d.focus + 1) % n,
            Key::Escape => {
                let act = match (self.pending.is_some(), self.editing.is_some()) {
                    (true, _) => Act::Reject,
                    (false, true) => Act::Cancel,
                    (false, false) => Act::Seen,
                };
                self.act(act, fetched);
            }
            Key::Enter => self.press(d.focus, fetched),
            Key::Left | Key::Right if d.focus < rows => self.press(d.focus, fetched),
            Key::Left => self.focus = rows + (d.focus - rows + buttons - 1) % buttons,
            Key::Right => self.focus = rows + (d.focus - rows + 1) % buttons,
        }
    }

    fn act(&mut self, act: Act, fetched: &mut Fetched) {
        self.focus = 0;
        match act {
            Act::Accept => {
                if let Some((flash, _)) = self.pending.take() {
                    self.take(&flash, fetched);
                }
            }
            Act::Reject => {
                if let Some((flash, _)) = self.pending.take() {
                    let report = self.deck.reject(&flash);
                    self.out.push(report);
                    self.rejected = flash.id.clone();
                    self.toast = Some((dialog::rejected(&self.locale, &ready::show_of(&flash)), self.now + TOAST_SECS));
                }
            }
            Act::Seen => self.deck.seen(),
            Act::Back => self.switch(true, fetched),
            // the conductor reads it with the deck's next word about itself: at once
            Act::Save => {
                if let Some(on) = self.editing.take() {
                    // a reason the deck came with (a native deck's deck.toml) stays; the switch gives none
                    let why = self.deck.keep().unwrap_or_default().to_string();
                    self.deck.set_keep(on.then_some(why));
                    let report = self.deck.about();
                    self.out.push(report);
                    self.toast = Some((dialog::keep_saved(&self.locale, self.deck.keep()), self.now + TOAST_SECS));
                }
            }
            Act::Cancel => self.editing = None,
        }
    }

    /// #471: the switch after a flash replaced the deck's own version: to its own (`true`) or back to the flashed one.
    /// Whether it may is the deck's (`ready::Deck::switch`); a toast says what runs again, or why not.
    pub fn switch(&mut self, own: bool, fetched: &mut Fetched) {
        match self.deck.switch(own) {
            Ok(show) => {
                let toast = if own { dialog::own_again(&self.locale, &show) } else { dialog::flashed_again(&self.locale, &show) };
                self.toast = Some((toast, self.now + TOAST_SECS));
                self.check(show, fetched);
            }
            Err(why) => self.toast = Some((dialog::failed(why), self.now + TOAST_SECS)),
        }
    }

    /// One turn (a frame) at `now` (the deck's clock, s): a suggestion nobody answered in time is accepted; a flash whose
    /// show is here asks for its pieces once; when every answer is here, the report.
    /// `carried`: what the deck has of a piece without asking (a built-in scene).
    pub fn turn(&mut self, now: f32, fetched: &mut Fetched, carried: impl Fn(&Place) -> Option<Found>) {
        self.now = now;
        if let Some((_, since)) = self.pending.as_mut() {
            if now - *since.get_or_insert(now) >= SUGGESTION_SECS {
                self.act(Act::Accept, fetched);
            }
        }
        let Some(c) = self.checking.as_mut() else { return };
        let told = self.deck.told(&c.show).map(<[ready::Need]>::to_vec);
        if told.is_none() && c.doc.is_none() {
            return;
        }
        if !c.asked {
            c.asked = true;
            let needs = told.unwrap_or_else(|| c.doc.as_ref().and_then(|d| d.as_ref().ok()).map(|d| ready::needs(d, &self.outlet)).unwrap_or_default());
            for need in needs {
                let _ = available(&need.piece, |p| carried(p).or_else(|| fetched.find(p)));
            }
        }
        if fetched.waiting() {
            return;
        }
        let Some(Checking { show, doc, .. }) = self.checking.take() else { return };
        let fetched = &*fetched;
        let has = |piece: &str| available(piece, |p| carried(p).or_else(|| fetched.have(p)));
        let report = match self.deck.check_told(&show, has) {
            Some(report) => report,
            None => self.deck.check(&show, doc.unwrap_or_else(|| Err("the show was not fetched".into())), has),
        };
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
    use crate::dialog::Tone;
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
        Flash { root: top.clone(), shows: vec![top], lock: None, hops: 0, here: None, id: Some(id.into()), by: None, skip: None, force: None, needs: None }
    }

    #[test]
    fn who_may_reject_is_the_role_live_in_the_decks_proof() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.joined("c1");
        // a member's deck is flashed: no question, and no "please don't flash"
        f.set_account(&["member".to_string()]);
        f.flash(&flash("f1"), API, "", &mut fetched);
        assert!(!matches!(f.shown(), Some(Shown::Dialog(_))) && !f.open_keep());
        // with `live` the next flash is a suggestion
        f.set_account(&["member".to_string(), "live".to_string()]);
        let mut next = flash("f2");
        (next.root.version, next.shows[0].version) = (4, 4);
        f.flash(&next, API, "", &mut fetched);
        assert!(matches!(f.shown(), Some(Shown::Dialog(d)) if d.buttons.len() == 2));
        f.key(Key::Escape, &mut fetched);
        assert!(f.open_keep());
        f.key(Key::Escape, &mut fetched);
        // a guest (no proof, or signed out): nothing of it
        f.set_account(&[]);
        assert!(!f.open_keep());
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
        f.turn(0.0, &mut fetched, carried);
        assert!(f.reports().is_empty(), "the show is on its way");
        assert!(!f.document("shows/other@1.json", Ok((200, SHOW))), "not asked for");
        // the API's answer as it is (`show`); then the piece the deck does not carry is asked for
        assert!(f.document("shows/top@3.json", Ok((200, &format!(r#"{{ "status": "success", "show": {SHOW} }}"#)))));
        f.turn(0.0, &mut fetched, carried);
        let boy = Place::new("character", "boy");
        assert_eq!(fetched.asks().len(), 1);
        f.turn(0.0, &mut fetched, carried);
        assert!(f.reports().is_empty(), "the piece is on its way");
        fetched.give(&boy, Ok(CONTENT));
        f.turn(0.0, &mut fetched, carried);
        let r = f.reports();
        assert_eq!((r.len(), r[0].ready, r[0].device.as_deref()), (1, Some(false), Some("browser")));
        assert_eq!(r[0].lacks.as_deref(), Some(&["row b: character:boy has no published rig".to_string()][..]));
        // the same show flashed again: kept, nothing is asked of the platform; the piece is asked for again
        f.flash(&flash("two"), API, "", &mut fetched);
        assert_eq!(f.ask(), None);
        f.turn(0.0, &mut fetched, carried);
        assert_eq!(fetched.asks().len(), 1);
    }

    /// #475: the conductor says what the outlet plays – a deck that may not read the show checks all the same
    #[test]
    fn a_flash_that_says_what_the_outlet_plays_needs_no_document() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        let mut told = flash("one");
        told.needs = Some(ready::flash_needs(&ready::read(SHOW).unwrap()));
        // not paired, no token: nothing is asked of the API
        f.flash(&told, API, "the deck is not paired yet", &mut fetched);
        assert_eq!(f.ask(), None);
        f.turn(0.0, &mut fetched, carried);
        assert_eq!(fetched.asks().len(), 1, "the piece it does not carry is asked for");
        assert!(f.reports().is_empty());
        fetched.give(&Place::new("character", "boy"), Ok(CONTENT));
        f.turn(0.0, &mut fetched, carried);
        let r = f.reports();
        assert_eq!((r[0].ready, r[0].lacks.as_deref()), (Some(false), Some(&["row b: character:boy has no published rig".to_string()][..])));
        // an outlet the show does not have: no cell will come
        let mut g = Flashed::new("foyer", "browser");
        g.flash(&told, API, "", &mut fetched);
        g.turn(0.0, &mut fetched, carried);
        assert_eq!(g.reports()[0].lacks.as_deref(), Some(&["top@3 has no outlet 'foyer': no cell will come".to_string()][..]));
        // a show with nothing missing is ready
        let mut h = Flashed::new("left", "browser");
        told.needs = Some([("left".to_string(), vec![protocol::FlashNeed { piece: "scene:cathedral".into(), row: "a".into() }])].into());
        h.flash(&told, API, "the deck is not paired yet", &mut fetched);
        h.turn(0.0, &mut fetched, carried);
        assert_eq!(h.reports()[0].ready, Some(true));
    }

    /// #481: the person at the deck – who may reject, the countdown, the notice, the line
    #[test]
    fn a_suggestion_waits_for_the_person_and_is_taken_when_nobody_answers() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.set_locale("de");
        f.joined("d1");
        f.reports();
        let mut told = flash("one");
        told.by = Some("ana".into());
        told.needs = Some([("left".to_string(), vec![protocol::FlashNeed { piece: "scene:cathedral".into(), row: "a".into() }])].into());
        // a guest or a member: flashed at once, one line that goes by itself
        f.flash(&told, API, "", &mut fetched);
        f.turn(1.0, &mut fetched, carried);
        assert_eq!(f.reports()[0].ready, Some(true));
        assert_eq!(f.shown(), Some(Shown::Toast("Geflasht von ana: top · Version 3".into(), Tone::Good)));
        f.turn(1.0 + TOAST_SECS, &mut fetched, carried);
        assert_eq!(f.shown(), None);

        // someone who may reject: the next version is a suggestion with a countdown
        f.set_may_reject(true);
        let mut next = told.clone();
        (next.id, next.root.version) = (Some("two".into()), 4);
        f.flash(&next, API, "", &mut fetched);
        f.turn(10.0, &mut fetched, carried);
        f.turn(14.0, &mut fetched, carried);
        let Some(Shown::Dialog(d)) = f.shown() else { panic!("the suggestion") };
        assert_eq!((d.title.as_str(), d.gone, d.buttons.len()), ("top · Version 4", Some(0.4), 2));
        assert!(f.reports().is_empty() && f.deck.runs().map(|r| r.version) == Some(3), "nothing is taken while it waits");
        // rejected: the conductor hears it, what runs stays, the same flash is not offered again
        f.key(Key::Right, &mut fetched);
        f.key(Key::Enter, &mut fetched);
        let r = f.reports();
        assert_eq!((r[0].accepted, r[0].why.as_deref(), r[0].running.version), (false, Some("rejected at the deck"), 3));
        assert_eq!(f.shown(), Some(Shown::Toast("Vorschlag abgelehnt: top · Version 4".into(), Tone::Warn)));
        f.flash(&next, API, "", &mut fetched);
        assert!(!matches!(f.shown(), Some(Shown::Dialog(_))));
        assert!(f.reports().iter().all(|r| !r.accepted), "a rejoin hears the same no");

        // nobody answers: after SUGGESTION_SECS it is taken
        let mut third = next.clone();
        (third.id, third.root.version) = (Some("three".into()), 5);
        f.flash(&third, API, "", &mut fetched);
        f.turn(30.0, &mut fetched, carried);
        f.turn(30.0 + SUGGESTION_SECS - 0.1, &mut fetched, carried);
        assert!(matches!(f.shown(), Some(Shown::Dialog(_))));
        f.turn(30.0 + SUGGESTION_SECS, &mut fetched, carried);
        assert_eq!(f.deck.runs().map(|r| r.version), Some(5));
        assert!(f.reports().iter().any(|r| r.accepted && r.running.version == 5));
    }

    #[test]
    fn a_forced_flash_is_taken_and_the_notice_offers_the_way_back() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.set_locale("de");
        f.set_may_reject(true);
        f.joined("d1");
        let mut first = flash("one");
        first.needs = Some([("left".to_string(), vec![])].into());
        first.force = Some(vec!["d1".into()]);
        f.flash(&first, API, "", &mut fetched);
        f.turn(0.0, &mut fetched, carried);
        assert!(matches!(f.shown(), Some(Shown::Toast(..))), "nothing of its own was replaced: a toast");
        // forced over what runs: no question, the notice with the way back
        let mut forced = first.clone();
        (forced.id, forced.root.version, forced.by) = (Some("two".into()), 4, Some("ana".into()));
        f.flash(&forced, API, "", &mut fetched);
        f.turn(1.0, &mut fetched, carried);
        assert_eq!(f.deck.runs().map(|r| r.version), Some(4));
        let Some(Shown::Dialog(d)) = f.shown() else { panic!("the notice") };
        assert_eq!((d.title.as_str(), d.buttons.len(), d.gone), ("top · Version 4 läuft", 2, None));
        assert_eq!(d.caption.0, "Flash · erzwungen");
        f.reports();
        f.press(1, &mut fetched);
        assert_eq!(f.deck.runs().map(|r| r.version), Some(3));
        // the way back checks from what the first flash said the outlet plays: no document is asked for
        assert_eq!(f.ask(), None);
        f.turn(2.0, &mut fetched, carried);
        assert!(f.reports().iter().any(|r| r.running.version == 3 && r.ready == Some(true)));
        assert_eq!(f.shown(), Some(Shown::Toast("Deine Version läuft wieder: top · Version 3".into(), Tone::Good)));
        // Escape on a notice reads it
        f.flash(&{ let mut again = forced.clone(); again.id = Some("three".into()); again }, API, "", &mut fetched);
        assert!(matches!(f.shown(), Some(Shown::Dialog(_))));
        f.key(Key::Escape, &mut fetched);
        assert!(!matches!(f.shown(), Some(Shown::Dialog(_))));
    }

    #[test]
    fn a_show_that_is_not_here_is_what_the_deck_lacks() {
        let (mut f, mut fetched) = (Flashed::new("left", "browser"), Fetched::default());
        f.flash(&flash("one"), API, "", &mut fetched);
        assert!(f.document("shows/top@3.json", Ok((403, r#"{ "status": "error", "message": "The role live is required." }"#))));
        f.turn(0.0, &mut fetched, carried);
        assert_eq!(f.reports()[0].lacks.as_deref(), Some(&["top@3: the show is not here (HTTP 403: The role live is required.)".to_string()][..]));
        // a deck without a token says why and asks nothing
        let mut g = Flashed::new("left", "browser");
        g.flash(&flash("one"), API, "the deck is not paired yet", &mut fetched);
        assert_eq!(g.ask(), None);
        g.turn(0.0, &mut fetched, carried);
        assert_eq!(g.reports()[0].lacks.as_deref(), Some(&["top@3: the show is not here (the deck is not paired yet)".to_string()][..]));
    }
}
