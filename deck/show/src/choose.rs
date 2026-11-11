//! Choosing what to flash (#471, SHOW.md §4b "Choosing what to flash", owner 2026-11-10 `[live]`): Flash opens a list of
//! every device of the conductor's tree, all chosen; a deck that asks not to be flashed starts unticked (a request, not a
//! lock). Quick filters and the typed filter set the ticks, a hand changes them. Nothing goes out before the prompt is
//! confirmed: it says how many get it, who loses unsaved configuration, who keeps local mappings, who is left out and why.
//! Once warned and accepted, a forced flash forces. A deck that is not here gets the flash when it joins, unless left out.
//! #481 (owner 2026-11-10: "wenn er forced. Wenn er nicht forced, wenn er nur als Suggestion schickt, dann kann man es
//! ablehnen"; 2026-11-11: "Force."): a flash goes out as a suggestion – who may reject at a deck has a few seconds to
//! say no – or forced, the list's switch: then every chosen deck takes it and nobody is asked. A deck that rejected
//! the last one says so in its line.
//!
//! The list, the filters, the prompt's words and what the flash then names (`Pick`) are here; the deck editor lays them out
//! and hands the presses in (RULES A1, A2).
use crate::document::Pinned;
use crate::subtree::Subtree;
use crate::validate::Lock;
use std::collections::BTreeSet;

/// What the confirmed prompt sends with the flash (`Subtree::flash_to`): the paths left out, the paths flashed although
/// the prompt warned, whether it is locked, who flashed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pick {
    pub skip: Vec<String>,
    pub force: Vec<String>,
    pub locked: bool,
    pub by: Option<String>,
}

/// One device of the tree, as its last report says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// a client id of this session, `foyer/c3` in a seat's session, `foyer` for the sub conductor in that seat
    pub path: String,
    /// `conductor`, `native`, `browser`, `phone` … (`deck` when it does not say)
    pub kind: String,
    pub seat: Option<String>,
    pub outlet: Option<String>,
    pub runs: Option<Pinned>,
    pub ready: Option<bool>,
    /// "please don't flash", and why
    pub keep: Option<String>,
    pub unsaved: bool,
    pub mappings: u64,
    pub online: bool,
    /// it runs its own kept version, not the flashed one
    pub own: bool,
    pub kept: Option<Pinned>,
    /// not on the version this flash pins
    pub old: bool,
    /// #481: the person at it rejected the last flash suggested to it
    pub rejected: bool,
}

impl Line {
    /// has configuration of its own: unsaved, or local mappings
    pub fn changed(&self) -> bool {
        self.unsaved || self.mappings > 0
    }

    /// what the list shows: `c7 · outlet left`, `foyer · sub conductor`
    pub fn head(&self) -> String {
        match &self.outlet {
            Some(o) => format!("{} · outlet {o}", self.path),
            None => format!("{} · sub conductor", self.path),
        }
    }

    /// what it runs and its state, in words
    pub fn state(&self) -> String {
        let mut parts = vec![match (&self.runs, self.own, &self.kept) {
            (Some(r), true, _) => format!("runs its own {r}"),
            (Some(r), false, Some(k)) => format!("runs {r} (its own {k} kept)"),
            (Some(r), false, None) => format!("runs {r}"),
            (None, _, _) => "no answer yet".to_string(),
        }];
        if !self.online {
            parts.push("not here".into());
        }
        match self.ready {
            Some(true) => parts.push("ready".into()),
            Some(false) => parts.push("lacks pieces".into()),
            None => {}
        }
        if self.unsaved {
            parts.push("unsaved configuration".into());
        }
        if self.mappings > 0 {
            parts.push(format!("{} local mappings", self.mappings));
        }
        if let Some(why) = &self.keep {
            parts.push(format!("asks not to be flashed: {why}"));
        }
        if self.rejected {
            parts.push("rejected the last flash".into());
        }
        parts.join(" · ")
    }
}

/// The quick filters: each sets the ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Quick {
    All,
    None,
    /// not on the version this flash pins
    Old,
    Ready,
    /// has local changes (unsaved configuration or local mappings)
    Changed,
    Kind(String),
    Seat(String),
}

/// How a line of the prompt reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    /// the operator is warned: confirming accepts it
    Warn,
    Muted,
}

/// The prompt before every flash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub title: String,
    pub lines: Vec<(String, Tone)>,
    /// something is forced: the confirm accepts the warning
    pub warn: bool,
    /// the confirm button's word
    pub confirm: String,
    /// anything chosen at all
    pub can: bool,
}

/// The flash list of one conductor: every device of its tree, the ticks, the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub lines: Vec<Line>,
    ticked: BTreeSet<String>,
    pub locked: bool,
    /// #481: forced – every chosen deck takes it, nobody at a deck is asked; else a suggestion
    pub forced: bool,
    /// the typed filter as last applied, and why it did not parse
    pub filter: String,
    pub error: Option<String>,
    target: Vec<Pinned>,
}

fn under(path: &str, conductor: &str) -> bool {
    path.strip_prefix(conductor).is_some_and(|rest| rest.starts_with('/'))
}

impl Choice {
    /// The list for a flash of `lock`: every device that said anything (decks, browser decks, sub conductors with their
    /// decks) and every seat of the document, all chosen but those that ask not to be flashed.
    pub fn open(sub: &Subtree, lock: &Lock) -> Choice {
        let target = lock.shows.clone();
        let mut lines: Vec<Line> = Vec::new();
        for (path, r) in sub.devices() {
            let (path, kind) = match &r.outlet {
                Some(_) => (path.clone(), r.device.clone().unwrap_or_else(|| "deck".to_string())),
                // a sub conductor's report comes as `seat/<its client>`: its line is the seat
                None => match path.rsplit_once('/') {
                    Some((seat, _)) => (seat.to_string(), "conductor".to_string()),
                    None => continue,
                },
            };
            if lines.iter().any(|l| l.path == path) {
                continue;
            }
            let runs = (!r.running.id.is_empty()).then(|| crate::flash::pinned(&r.running));
            let old = runs.as_ref().is_none_or(|p| target.iter().find(|t| t.id == p.id).is_none_or(|t| t.version != p.version));
            let seat = path.split_once('/').map(|(s, _)| s.to_string()).or_else(|| (kind == "conductor").then(|| path.clone()));
            lines.push(Line {
                online: sub.is_online(&path),
                outlet: r.outlet.clone(),
                ready: r.ready,
                keep: r.keep.clone(),
                unsaved: r.unsaved == Some(true),
                mappings: r.mappings.unwrap_or(0),
                own: r.own == Some(true),
                kept: r.kept.as_ref().map(crate::flash::pinned),
                rejected: !r.accepted && r.why.as_deref() == Some(crate::ready::REJECTED),
                path,
                kind,
                seat,
                runs,
                old,
            });
        }
        for s in &sub.conductor().show().seats {
            if !lines.iter().any(|l| l.path == s.id) {
                lines.push(Line {
                    path: s.id.clone(),
                    kind: "conductor".into(),
                    seat: Some(s.id.clone()),
                    outlet: None,
                    runs: None,
                    ready: None,
                    keep: None,
                    unsaved: false,
                    mappings: 0,
                    online: sub.is_online(&s.id),
                    own: false,
                    kept: None,
                    old: true,
                    rejected: false,
                });
            }
        }
        lines.sort_by(|a, b| (a.seat.is_some(), &a.path).cmp(&(b.seat.is_some(), &b.path)));
        let ticked = lines.iter().filter(|l| l.keep.is_none()).map(|l| l.path.clone()).collect();
        Choice { lines, ticked, locked: false, forced: false, filter: String::new(), error: None, target }
    }

    /// While the list is open, what the devices say changes (one joins, one answers): the lines follow, a line's tick
    /// stays as it was set, a new line starts as `open` would start it.
    pub fn refresh(&mut self, sub: &Subtree) {
        let lock = Lock { shows: self.target.clone() };
        let fresh = Choice::open(sub, &lock);
        let known: BTreeSet<String> = self.lines.iter().map(|l| l.path.clone()).collect();
        let mut ticked: BTreeSet<String> = fresh.ticked.iter().filter(|p| !known.contains(*p)).cloned().collect();
        ticked.extend(self.ticked.iter().filter(|p| fresh.lines.iter().any(|l| &l.path == *p)).cloned());
        self.lines = fresh.lines;
        self.ticked = ticked;
    }

    pub fn is_ticked(&self, i: usize) -> bool {
        self.lines.get(i).is_some_and(|l| self.ticked.contains(&l.path))
    }

    pub fn count(&self) -> usize {
        self.lines.iter().filter(|l| self.ticked.contains(&l.path)).count()
    }

    fn set(&mut self, path: &str, on: bool) {
        // a sub conductor carries what sits in its seat with it
        let paths: Vec<String> = self.lines.iter().filter(|l| l.path == path || under(&l.path, path)).map(|l| l.path.clone()).collect();
        for p in paths {
            if on {
                self.ticked.insert(p);
            } else {
                self.ticked.remove(&p);
            }
        }
    }

    /// A hand ticks or unticks line `i` (a sub conductor with its decks).
    pub fn toggle(&mut self, i: usize) {
        let Some(path) = self.lines.get(i).map(|l| l.path.clone()) else { return };
        let on = !self.ticked.contains(&path);
        self.set(&path, on);
    }

    /// The quick filters for this list, with their words: all, none, the states, each kind and each seat there is.
    pub fn quicks(&self) -> Vec<(Quick, String)> {
        let mut out = vec![
            (Quick::All, "all".to_string()),
            (Quick::None, "none".to_string()),
            (Quick::Old, "not on this version".to_string()),
            (Quick::Ready, "ready".to_string()),
            (Quick::Changed, "has local changes".to_string()),
        ];
        let kinds: BTreeSet<&String> = self.lines.iter().map(|l| &l.kind).collect();
        out.extend(kinds.into_iter().map(|k| (Quick::Kind(k.clone()), k.clone())));
        let seats: BTreeSet<&String> = self.lines.iter().filter_map(|l| l.seat.as_ref()).collect();
        out.extend(seats.into_iter().map(|s| (Quick::Seat(s.clone()), format!("seat {s}"))));
        out
    }

    fn tick_where(&mut self, keep: impl Fn(&Line) -> bool) {
        self.ticked = self.lines.iter().filter(|l| keep(l)).map(|l| l.path.clone()).collect();
    }

    /// A quick filter sets the ticks (the hand can change them after).
    pub fn quick(&mut self, q: &Quick) {
        match q {
            Quick::All => self.tick_where(|_| true),
            Quick::None => self.tick_where(|_| false),
            Quick::Old => self.tick_where(|l| l.old),
            Quick::Ready => self.tick_where(|l| l.ready == Some(true)),
            Quick::Changed => self.tick_where(Line::changed),
            Quick::Kind(k) => self.tick_where(|l| &l.kind == k),
            Quick::Seat(s) => self.tick_where(|l| l.seat.as_ref() == Some(s)),
        }
    }

    /// The typed filter (SHOW.md §9): `where device:browser and group:workshop`. Sets the ticks to what matches; `Err` = why
    /// it does not read (the ticks stay).
    pub fn filter(&mut self, text: &str) -> Result<usize, String> {
        self.filter = text.to_string();
        match terms(text) {
            Ok(terms) => {
                self.error = None;
                self.tick_where(|l| terms.iter().all(|t| t.matches(l)));
                Ok(self.count())
            }
            Err(e) => {
                self.error = Some(e.clone());
                Err(e)
            }
        }
    }

    /// The prompt before the flash: always, also for one deck.
    pub fn prompt(&self) -> Prompt {
        let n = self.count();
        let root = self.target.first().map_or("the show".to_string(), ToString::to_string);
        if n == 0 {
            return Prompt { title: "Nothing is chosen: tick a device to flash".into(), lines: vec![], warn: false, confirm: "Flash".into(), can: false };
        }
        let mut lines = vec![];
        let mut warn = false;
        for l in &self.lines {
            if !self.ticked.contains(&l.path) {
                let why = match (&l.keep, l.kind == "conductor") {
                    (Some(why), _) => format!(": asks not to be flashed ({why})"),
                    (None, true) => " with everything in its seat".to_string(),
                    (None, false) => String::new(),
                };
                lines.push((format!("{} is left out{why}", l.path), Tone::Muted));
                continue;
            }
            if let Some(why) = &l.keep {
                warn = true;
                lines.push((format!("{} asks not to be flashed ({why}) – flashed anyway, its own version is kept", l.path), Tone::Warn));
            }
            if l.unsaved {
                warn = true;
                lines.push((format!("{} loses its unsaved configuration – kept as its own copy", l.path), Tone::Warn));
            }
            if l.mappings > 0 {
                lines.push((format!("{} keeps its {} local mappings", l.path, l.mappings), Tone::Plain));
            }
            if l.own {
                lines.push((format!("{} runs its own version – goes to {root}", l.path), Tone::Plain));
            }
            if l.rejected {
                lines.push(if self.forced { (format!("{} rejected the last flash – flashed anyway", l.path), Tone::Warn) } else { (format!("{} rejected the last flash – asked again", l.path), Tone::Plain) });
            }
            if !l.online {
                lines.push((format!("{} is not here – gets it when it joins", l.path), Tone::Muted));
            }
        }
        lines.push(("A deck that joins later gets it too, unless it asks not to be flashed.".into(), Tone::Muted));
        lines.push(if self.forced {
            warn = true;
            ("Forced: every chosen deck takes it, nobody at a deck is asked.".into(), Tone::Warn)
        } else {
            (format!("A suggestion: who may reject at a deck has {} s to say no.", crate::ready::SUGGESTION_SECS), Tone::Muted)
        });
        lines.push(if self.locked {
            ("Locked: only a deck's owner can go back to its own version.".into(), Tone::Plain)
        } else {
            ("Each deck's operator can go back to their own version.".into(), Tone::Muted)
        });
        let total = self.lines.len();
        let what = if n == 1 { "device" } else { "devices" };
        Prompt { title: format!("Flash {root} to {n} of {total} {what}?"), lines, warn, confirm: if warn { "Force flash".into() } else { "Flash".into() }, can: true }
    }

    /// What the confirmed prompt sends: the unticked are left out; forced are the ticked that were warned of, or every
    /// ticked one when the flash is forced.
    pub fn pick(&self, by: Option<String>) -> Pick {
        let (on, off): (Vec<&Line>, Vec<&Line>) = self.lines.iter().partition(|l| self.ticked.contains(&l.path));
        Pick {
            skip: off.iter().map(|l| l.path.clone()).collect(),
            force: on.iter().filter(|l| self.forced || l.keep.is_some() || l.unsaved).map(|l| l.path.clone()).collect(),
            locked: self.locked,
            by,
        }
    }
}

/// One condition of the typed filter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Term {
    not: bool,
    key: String,
    value: Option<String>,
}

/// What the typed filter's empty field shows.
pub const FILTER_HINT: &str = "where device:browser and group:workshop";

const KEYS: &str ="device, kind, group, seat, outlet, show, or one of ready, changed, old, keep, here, own, rejected";

fn terms(text: &str) -> Result<Vec<Term>, String> {
    let text = text.trim();
    let text = text.strip_prefix("where").map_or(text, str::trim_start);
    if text.is_empty() {
        return Ok(vec![]);
    }
    text.split(" and ")
        .map(|t| {
            let t = t.trim().to_lowercase();
            let (not, t) = t.strip_prefix("not ").map_or((false, t.as_str()), |r| (true, r.trim()));
            let (key, value) = match t.split_once(':') {
                Some((k, v)) => (k.trim().to_string(), Some(v.trim().to_string())),
                None => (t.to_string(), None),
            };
            let known = match &value {
                Some(_) => ["device", "kind", "group", "seat", "outlet", "show"].contains(&key.as_str()),
                None => ["ready", "changed", "old", "keep", "here", "own", "rejected"].contains(&key.as_str()),
            };
            if !known || t.is_empty() {
                return Err(format!("'{t}' is no condition: {KEYS}"));
            }
            Ok(Term { not, key, value })
        })
        .collect()
}

impl Term {
    fn matches(&self, l: &Line) -> bool {
        let eq = |s: Option<&String>| s.is_some_and(|s| Some(s.to_lowercase()) == self.value);
        let yes = match self.key.as_str() {
            "device" | "kind" => eq(Some(&l.kind)),
            "group" => eq(l.seat.as_ref()) || eq(l.outlet.as_ref()),
            "seat" => eq(l.seat.as_ref()),
            "outlet" => eq(l.outlet.as_ref()),
            "show" => eq(l.runs.as_ref().map(|p| &p.id)),
            "ready" => l.ready == Some(true),
            "changed" => l.changed(),
            "old" => l.old,
            "keep" => l.keep.is_some(),
            "here" => l.online,
            "own" => l.own,
            "rejected" => l.rejected,
            _ => false,
        };
        yes != self.not
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClockChoice, Show};
    use protocol::{Envelope, FlashReport, Message};

    const TOP: &str = r#"{ "id": "top", "version": 3, "outlets": [{ "id": "left" }, { "id": "right" }],
      "seats": [{ "id": "foyer", "show": { "id": "foyer-loop", "version": 2 }, "word": "follow" }],
      "rows": [{ "id": "start", "cells": { "left": { "mode": "black" } } }] }"#;

    fn report(running: (&str, u32), outlet: Option<&str>) -> FlashReport {
        let mut r = crate::flash::report(&Pinned { id: running.0.into(), version: running.1 }, true, None);
        r.outlet = outlet.map(str::to_string);
        r
    }

    fn sub() -> Subtree {
        let show: Show = crate::parse(TOP).unwrap();
        let mut sub = Subtree::new(show, ClockChoice::Parent);
        let mut c7 = report(("top", 3), Some("left"));
        c7.ready = Some(true);
        c7.device = Some("native".into());
        let mut c8 = report(("top", 2), Some("right"));
        c8.keep = Some("rehearsing".into());
        c8.device = Some("browser".into());
        let mut c3 = report(("foyer-loop", 1), Some("hall"));
        c3.unsaved = Some(true);
        c3.mappings = Some(2);
        for (from, r) in [("c7", c7), ("c8", c8), ("foyer/c1", report(("foyer-loop", 2), None)), ("foyer/c3", c3)] {
            sub.on_wire(&Envelope::new(from, Message::FlashReport(r)), 10);
        }
        let presence = serde_json::json!({ "groups": {}, "ready": [], "links": { "c7": { "hz": 30 }, "c8": { "hz": 30 } }, "seats": { "foyer": { "online": true } } });
        sub.on_wire(&Envelope::new("relay", Message::Presence(serde_json::from_value(presence).unwrap())), 11);
        sub
    }

    fn lock() -> Lock {
        Lock { shows: vec![Pinned { id: "top".into(), version: 3 }, Pinned { id: "foyer-loop".into(), version: 2 }] }
    }

    #[test]
    fn every_device_of_the_tree_all_chosen_but_the_one_that_asks_not_to_be_flashed() {
        let c = Choice::open(&sub(), &lock());
        let paths: Vec<&str> = c.lines.iter().map(|l| l.path.as_str()).collect();
        assert_eq!(paths, ["c7", "c8", "foyer", "foyer/c3"]);
        let ticks: Vec<bool> = (0..4).map(|i| c.is_ticked(i)).collect();
        assert_eq!(ticks, [true, false, true, true]);
        assert_eq!(c.lines[2].kind, "conductor");
        assert_eq!(c.lines[1].state(), "runs top@2 · asks not to be flashed: rehearsing");
        assert_eq!(c.lines[3].state(), "runs foyer-loop@1 · unsaved configuration · 2 local mappings");
        assert_eq!(c.lines[1].head(), "c8 · outlet right");
    }

    #[test]
    fn a_conductor_that_joins_later_finds_its_decks_in_the_welcome() {
        let mut s = Subtree::new(crate::parse(TOP).unwrap(), ClockChoice::Parent);
        let welcome = serde_json::json!({ "client": "c1", "now": 0, "snapshot": { "reports": { "c7": { "running": { "id": "top", "version": 3 }, "accepted": true, "outlet": "left", "keep": "rehearsing" } } } });
        s.on_wire(&Envelope::new("", Message::Welcome(serde_json::from_value(welcome).unwrap())), 1);
        let c = Choice::open(&s, &lock());
        assert_eq!(c.lines[0].keep.as_deref(), Some("rehearsing"));
        assert!(!c.is_ticked(0));
    }

    #[test]
    fn the_open_list_follows_the_devices_and_keeps_the_hands_ticks() {
        let mut s = sub();
        let mut c = Choice::open(&s, &lock());
        c.toggle(0); // c7 off by hand
        s.on_wire(&Envelope::new("c9", Message::FlashReport(report(("top", 3), Some("left")))), 20);
        c.refresh(&s);
        assert_eq!(c.lines.len(), 5);
        let c9 = c.lines.iter().position(|l| l.path == "c9").unwrap();
        assert!(!c.is_ticked(0) && c.is_ticked(c9), "the hand's tick stays, a new deck starts chosen");
    }

    #[test]
    fn quick_filters_set_the_ticks_and_the_hand_changes_them() {
        let mut c = Choice::open(&sub(), &lock());
        c.quick(&Quick::Old);
        assert_eq!(c.pick(None).skip, ["c7", "foyer"], "c8 runs top@2, foyer/c3 foyer-loop@1");
        c.quick(&Quick::Changed);
        assert_eq!(c.count(), 1);
        c.quick(&Quick::Kind("browser".into()));
        assert!(c.is_ticked(1));
        c.quick(&Quick::Seat("foyer".into()));
        assert_eq!(c.count(), 2);
        c.toggle(2);
        assert_eq!(c.count(), 0, "unticking a sub conductor unticks what sits in its seat");
        c.quick(&Quick::All);
        assert_eq!(c.count(), 4);
        assert!(c.quicks().iter().any(|(q, w)| *q == Quick::Seat("foyer".into()) && w == "seat foyer"));
    }

    #[test]
    fn the_typed_filter_sets_the_ticks_or_says_why_not() {
        let mut c = Choice::open(&sub(), &lock());
        assert_eq!(c.filter("where device:browser"), Ok(1));
        assert_eq!(c.filter("where group:foyer and not changed"), Ok(1), "the sub conductor");
        assert_eq!(c.filter("outlet:left and ready"), Ok(1));
        let e = c.filter("where colour:red").unwrap_err();
        assert!(e.starts_with("'colour:red' is no condition"), "{e}");
        assert_eq!(c.count(), 1, "the ticks stay");
    }

    #[test]
    fn the_prompt_says_how_many_who_loses_who_keeps_and_who_is_left_out() {
        let mut c = Choice::open(&sub(), &lock());
        let p = c.prompt();
        assert_eq!(p.title, "Flash top@3 to 3 of 4 devices?");
        assert!(p.warn, "foyer/c3 has unsaved configuration");
        assert_eq!(p.confirm, "Force flash");
        let words: Vec<&str> = p.lines.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            words,
            [
                "c8 is left out: asks not to be flashed (rehearsing)",
                "foyer/c3 loses its unsaved configuration – kept as its own copy",
                "foyer/c3 keeps its 2 local mappings",
                "A deck that joins later gets it too, unless it asks not to be flashed.",
                "A suggestion: who may reject at a deck has 10 s to say no.",
                "Each deck's operator can go back to their own version.",
            ]
        );
        c.toggle(1);
        c.locked = true;
        let p = c.prompt();
        assert_eq!(p.lines[0], ("c8 asks not to be flashed (rehearsing) – flashed anyway, its own version is kept".to_string(), Tone::Warn));
        assert_eq!(p.lines.last().unwrap().0, "Locked: only a deck's owner can go back to its own version.");
        let pick = c.pick(Some("ana".into()));
        assert_eq!((pick.skip.len(), pick.force.clone(), pick.locked), (0, vec!["c8".to_string(), "foyer/c3".to_string()], true));
        c.quick(&Quick::None);
        assert!(!c.prompt().can);
    }

    #[test]
    fn a_forced_flash_names_every_chosen_deck_and_a_deck_that_rejected_says_so() {
        let mut s = sub();
        // c7's person rejected the last suggestion
        let mut no = report(("top", 2), Some("left"));
        (no.accepted, no.why) = (false, Some(crate::ready::REJECTED.to_string()));
        s.on_wire(&Envelope::new("c7", Message::FlashReport(no)), 12);
        let mut c = Choice::open(&s, &lock());
        assert!(c.lines[0].rejected && c.is_ticked(0), "a rejection is no request not to be flashed: it stays chosen");
        assert_eq!(c.lines[0].state(), "runs top@2 · rejected the last flash");
        // as a suggestion: it is asked again, and only what the prompt warned of is forced
        let p = c.prompt();
        assert!(p.lines.contains(&("c7 rejected the last flash – asked again".to_string(), Tone::Plain)));
        assert_eq!(c.pick(None).force, ["foyer/c3"]);
        // forced: the prompt warns, and every chosen deck is named
        c.forced = true;
        let p = c.prompt();
        assert!(p.warn && p.confirm == "Force flash");
        assert!(p.lines.contains(&("c7 rejected the last flash – flashed anyway".to_string(), Tone::Warn)));
        assert!(p.lines.contains(&("Forced: every chosen deck takes it, nobody at a deck is asked.".to_string(), Tone::Warn)));
        assert_eq!(c.pick(None).force, ["c7", "foyer", "foyer/c3"]);
    }

    #[test]
    fn the_confirmed_pick_goes_down_and_a_seat_left_out_gets_nothing() {
        let mut s = sub();
        let mut c = Choice::open(&s, &lock());
        c.toggle(2); // foyer and its decks
        let out = s.flash_to(&lock(), &c.pick(Some("ana".into())), 100);
        let Message::Flash(f) = &out[0].message else { panic!("{out:?}") };
        assert_eq!(f.skip.as_deref(), Some(&["c8".to_string(), "foyer-loop".into(), "foyer-loop/c3".into()][..]));
        assert_eq!(f.by.as_deref(), Some("ana"));
        assert_eq!(f.id.as_deref(), Some("top@3/100"));
        // #475: the conductor holds the pinned version, so the flash says what each outlet plays
        assert!(f.needs.as_ref().is_some_and(|n| !n.is_empty()), "{:?}", f.needs);
        // the child conductor of foyer-loop refuses what left it out, and sends nothing down
        let mut child = Subtree::new(crate::parse(r#"{ "id": "foyer-loop", "version": 2, "outlets": [], "rows": [] }"#).unwrap(), ClockChoice::Parent);
        let back = child.on_wire(&Envelope::new(protocol::PARENT, Message::Flash(f.clone())), 101);
        assert_eq!(back.len(), 1);
        let Message::FlashReport(r) = &back[0].message else { panic!() };
        assert_eq!(r.why.as_deref(), Some("left out by the conductor above"));
    }
}
