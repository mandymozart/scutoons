//! A deck's side of a flash (#91 item 5, SHOW.md §4b): a deck takes the flash by the same rule as a conductor
//! ([`flash::take`]), keeps the show its outlet plays at the version the flash pins (the adapter's cache:
//! [`kept_as`]), checks every piece its outlet's cells name and answers with a `flash.report`: ready, or what it lacks, in
//! words. A broken or missing piece is a reason on the HUD and in the report; it never blocks GO (SHOW.md §7) – the
//! conductor fires on, the deck compares each cell as it comes (deck-space `piece.rs`).
//!
//! Sans IO: the adapter fetches the document (the API, else what it kept) and says whether it carries a piece; the rule,
//! what is needed and the words are here, once.
use crate::document::{self, Cell, Pinned, Show};
use crate::flash::{self, Assignment};
use protocol::{Flash, FlashReport};
use std::fmt;

/// How long a flash that is only suggested waits for the person at the deck (#481; owner 2026-11-10: "nach 10 Sekunden
/// geht der Dialog automatisch weg … wird es automatisch angenommen"): then it is taken – flashing is an explicit act.
pub const SUGGESTION_SECS: f32 = 10.0;

/// One piece an outlet's cells name, and the first row that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub piece: String,
    pub row: String,
}

/// The pieces `outlet` plays in `show`, each once, in row order. A `values` cell names the piece it expects too; a mirror
/// plays what its outlet plays in that row. A device outlet names none (it plays what a row sends).
pub fn needs(show: &Show, outlet: &str) -> Vec<Need> {
    let mut out: Vec<Need> = Vec::new();
    for row in &show.rows {
        let cell = match row.cells.get(outlet) {
            Some(Cell::Mirror { of }) => row.cells.get(of),
            other => other,
        };
        let piece = match cell {
            Some(Cell::Play { piece: Some(p), .. }) | Some(Cell::Values { piece: p, .. }) => p,
            _ => continue,
        };
        if !out.iter().any(|n| &n.piece == piece) {
            out.push(Need { piece: piece.clone(), row: row.id.clone() });
        }
    }
    out
}

/// What every outlet of `show` plays, for the flash that goes into its session (#475): a deck checks its pieces from
/// this, without reading the document. A device outlet is listed with none.
pub fn flash_needs(show: &Show) -> std::collections::HashMap<String, Vec<protocol::FlashNeed>> {
    show.outlets.iter().map(|o| (o.id.clone(), needs(show, &o.id).into_iter().map(|n| protocol::FlashNeed { piece: n.piece, row: n.row }).collect())).collect()
}

/// The show a flash assigns in the session it is in now: its `here` (the conductor of this deck's session), else its root.
pub fn show_of(flash: &Flash) -> Pinned {
    flash::pinned(flash.here.as_ref().unwrap_or(&flash.root))
}

/// What a deck lacks for a flash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lack {
    /// the show's document is neither fetched nor kept
    Show { show: Pinned, why: String },
    /// the show has no such outlet: no cell will come
    Outlet { show: Pinned, outlet: String },
    /// a piece the outlet's cells name does not play here (not on the deck, or broken)
    Piece { piece: String, row: String, why: String },
}

impl fmt::Display for Lack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lack::Show { show, why } => write!(f, "{show}: the show is not here ({why})"),
            Lack::Outlet { show, outlet } => write!(f, "{show} has no outlet '{outlet}': no cell will come"),
            // the piece's own reason names it already (deck-space piece.rs: "scene:x is not on this deck")
            Lack::Piece { piece, row, why } if why.contains(piece.as_str()) => write!(f, "row {row}: {why}"),
            Lack::Piece { piece, row, why } => write!(f, "row {row}: {piece}: {why}"),
        }
    }
}

/// The row a report's lack line points at (`row exit: …` → `exit`), so the editor's problems list can select it.
pub fn lack_row<'a>(line: &'a str, show: &Show) -> Option<&'a str> {
    let rest = line.strip_prefix("row ")?;
    show.rows.iter().map(|r| r.id.as_str()).find(|id| rest.starts_with(&format!("{id}: "))).map(|id| &rest[..id.len()])
}

/// The file a deck keeps a flashed show in, under its cache: one per exact version (a version never changes).
pub fn kept_as(p: &Pinned) -> String {
    format!("shows/{}@{}.json", p.id, p.version)
}

/// A show document as the API gives it – `GET /api/shows/<id>?v=<n>`: the version a flash pins, in `show`;
/// `GET /api/shows/<id>`: wrapped in `content` – or as it is kept (bare).
pub fn read(text: &str) -> Result<Show, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let doc = match v.get("show").or_else(|| v.get("content")) {
        Some(c) if c.is_object() => c.clone(),
        _ => v,
    };
    document::parse(&doc.to_string()).map_err(|e| e.to_string())
}

/// #471: what a flash replaced on this deck, kept as the deck's own copy, and who flashed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// the deck's own version (what ran before the first flash that replaced it)
    pub own: Pinned,
    /// the version the flash brought
    pub flashed: Pinned,
    pub by: String,
    /// it asked not to be flashed or had unsaved configuration, and the conductor's operator accepted the warning
    pub forced: bool,
    /// the flash was locked: only the deck's owner may go back
    pub locked: bool,
    /// the deck runs its own copy now
    pub on_own: bool,
}

/// The flash side of one deck's outlet.
#[derive(Debug, Clone)]
pub struct Deck {
    outlet: String,
    standing: Option<Assignment>,
    /// the show this outlet plays from, as the last flash it took assigned it
    running: Option<Pinned>,
    last: Option<FlashReport>,
    /// #471: this deck's client id in its session (the relay's welcome): the flash's lists name it so
    id: Option<String>,
    /// #471: "please don't flash", and why (a request: a forced flash goes over it)
    keep: Option<String>,
    /// #471: configuration a flash would replace, and the local mappings that stay
    unsaved: bool,
    mappings: u64,
    /// #471: native, browser, phone … (the conductor's filter)
    device: Option<String>,
    /// #471: the person at the deck is its owner (always allowed back to the own copy)
    owner: bool,
    /// #471: the flash taken last (one flash is taken once)
    taken: Option<String>,
    kept: Option<Kept>,
    /// #471: the notice was seen
    seen: bool,
    /// #475: what the last flash said this outlet plays in the show it assigned (none for the outlet: the show has no
    /// such outlet); absent when the flash did not say
    needed: Option<(Pinned, Option<Vec<Need>>)>,
}

impl Deck {
    pub fn new(outlet: &str) -> Deck {
        Deck { outlet: outlet.to_string(), standing: None, running: None, last: None, id: None, keep: None, unsaved: false, mappings: 0, device: None, owner: true, taken: None, kept: None, seen: false, needed: None }
    }

    fn running(&self) -> Pinned {
        self.running.clone().unwrap_or(Pinned { id: String::new(), version: 0 })
    }

    /// #471: the client id the relay gave this deck.
    pub fn set_id(&mut self, id: &str) {
        self.id = Some(id.to_string());
    }

    /// #471: "please don't flash" with a line of why (none = flash me). Said to the conductor with the next report.
    pub fn set_keep(&mut self, why: Option<String>) {
        self.keep = why.map(|w| w.trim().to_string()).map(|w| if w.is_empty() { "no reason given".to_string() } else { w });
    }

    pub fn keep(&self) -> Option<&str> {
        self.keep.as_deref()
    }

    /// #471: what the deck says about its configuration (the adapter knows it).
    pub fn set_configuration(&mut self, unsaved: bool, mappings: u64) {
        self.unsaved = unsaved;
        self.mappings = mappings;
    }

    pub fn set_device(&mut self, device: &str) {
        self.device = Some(device.to_string());
    }

    /// #471: whether the person at the deck is its owner (proposal, owner 2026-11-10 "Move on now": the owner may always go
    /// back, anyone else unless the flash was locked).
    pub fn set_owner(&mut self, owner: bool) {
        self.owner = owner;
    }

    pub fn kept(&self) -> Option<&Kept> {
        self.kept.as_ref()
    }

    /// #471: the facts a deck says about itself, on every report.
    fn stamp(&self, mut r: FlashReport) -> FlashReport {
        r.outlet = Some(self.outlet.clone());
        r.keep = self.keep.clone();
        r.unsaved = self.unsaved.then_some(true);
        r.mappings = (self.mappings > 0).then_some(self.mappings);
        r.device = self.device.clone();
        r.kept = self.kept.as_ref().map(|k| flash::wire(&k.own));
        r.own = self.kept.as_ref().map(|k| k.on_own);
        r
    }

    /// #481: whether `flash` is only suggested to this deck – one it has not taken, is not left out of, is not forced
    /// onto (the conductor's operator accepted the warning for it) and that would change what runs.
    pub fn suggested(&self, flash: &Flash) -> bool {
        let me = self.id.as_deref().unwrap_or_default();
        let taken = flash.id.is_some() && flash.id == self.taken;
        !taken && !flash::left_out(flash, me) && !flash::forced(flash, me) && self.running.as_ref() != Some(&show_of(flash))
    }

    /// #481: the person at the deck rejected a suggested flash: what runs stays, the conductor hears it, and the same
    /// flash is not offered again (a rejoin hands it over once more).
    pub fn reject(&mut self, flash: &Flash) -> FlashReport {
        self.taken = flash.id.clone();
        self.refuse("rejected at the deck".to_string())
    }

    /// what runs, when something does
    pub fn runs(&self) -> Option<&Pinned> {
        self.running.as_ref().filter(|r| !r.id.is_empty())
    }

    /// #481: the notice about what a flash replaced was not read yet
    pub fn unseen(&self) -> bool {
        self.kept.is_some() && !self.seen
    }

    fn refuse(&mut self, why: String) -> FlashReport {
        let r = self.stamp(flash::report(&self.running(), false, Some(why)));
        self.last = Some(r.clone());
        r
    }

    /// #471: what the deck says about itself when nothing new happened (it joined, its mark changed, it switched): the last
    /// report with today's facts, else what runs.
    pub fn about(&mut self) -> FlashReport {
        let base = self.last.clone().map(|mut r| {
            r.running = flash::wire(&self.running());
            r
        });
        let r = self.stamp(base.unwrap_or_else(|| flash::report(&self.running(), true, None)));
        self.last = Some(r.clone());
        r
    }

    /// A flash arrives. `Ok` = taken: the show to fetch (or read from what was kept) and then [`Deck::check`]; `Err` = the
    /// report of a refusal, to send at once. The show is the flash's `here` (the conductor of this deck's session), else its
    /// root.
    ///
    /// #471: a flash is taken once (a rejoin hands the same one again: what runs stays, the own copy too); a deck the
    /// operator left out refuses it; a deck that asks not to be flashed or has unsaved configuration takes it only when the
    /// operator was warned and accepted (`force`). A deck that was not here when it went out takes it when it joins. What
    /// it replaces is kept as the deck's own copy.
    pub fn take(&mut self, flash: &Flash) -> Result<Pinned, Box<FlashReport>> {
        if flash.id.is_some() && flash.id == self.taken {
            return Ok(self.running());
        }
        let me = self.id.clone().unwrap_or_default();
        if flash::left_out(flash, &me) {
            return Err(Box::new(self.refuse("left out by the conductor".to_string())));
        }
        let forced = flash::forced(flash, &me);
        if !forced {
            if let Some(why) = self.keep.clone() {
                return Err(Box::new(self.refuse(format!("asks not to be flashed: {why}"))));
            }
            if self.unsaved {
                return Err(Box::new(self.refuse("has unsaved configuration".to_string())));
            }
        }
        let running = self.running();
        let report = flash::take(&mut self.standing, flash, &running, None);
        if !report.accepted {
            let r = self.stamp(report);
            self.last = Some(r.clone());
            return Err(Box::new(r));
        }
        let here = flash.here.as_ref().unwrap_or(&flash.root);
        let show = flash::pinned(here);
        self.taken = flash.id.clone();
        // the own copy: what ran before the first flash that replaced it (a later flash replaces only a flashed version)
        let own = match &self.kept {
            Some(k) if !k.on_own => Some(k.own.clone()),
            _ if !running.id.is_empty() && running != show => Some(running),
            _ if self.unsaved => Some(running),
            Some(k) => Some(k.own.clone()),
            None => None,
        };
        self.kept = own.map(|own| Kept {
            own,
            flashed: show.clone(),
            by: flash.by.clone().unwrap_or_else(|| "the conductor".to_string()),
            forced: forced && (self.keep.is_some() || self.unsaved),
            locked: flash.lock == Some(true),
            on_own: false,
        });
        self.seen = self.kept.is_none();
        self.unsaved = false;
        self.running = Some(show.clone());
        self.needed = flash.needs.as_ref().map(|by_outlet| {
            let mine = by_outlet.get(&self.outlet).map(|n| n.iter().map(|n| Need { piece: n.piece.clone(), row: n.row.clone() }).collect());
            (show.clone(), mine)
        });
        Ok(show)
    }

    /// #471: whether the person at the deck may switch between the flashed version and its own: the owner always, anyone
    /// else unless the flash was locked (proposal).
    pub fn may_switch(&self) -> bool {
        self.kept.as_ref().is_some_and(|k| self.owner || !k.locked)
    }

    /// #471: go back to the own copy (`true`) or to the flashed version: the show to play now (then [`Deck::check`]), or why
    /// not.
    pub fn switch(&mut self, to_own: bool) -> Result<Pinned, String> {
        let Some(k) = self.kept.clone() else { return Err("nothing of this deck's own is kept".to_string()) };
        if !self.may_switch() {
            return Err(format!("the flash by {} is locked: only the deck's owner can go back", k.by));
        }
        let show = if to_own { k.own.clone() } else { k.flashed.clone() };
        self.kept = Some(Kept { on_own: to_own, ..k });
        self.running = Some(show.clone());
        self.seen = true;
        Ok(show)
    }

    /// #471: the operator has seen the notice.
    pub fn seen(&mut self) {
        self.seen = true;
    }

    /// #471: the notice on the deck after a flash replaced its own version: who, whether by force, that the own one is kept,
    /// and how the switch stands. Empty when nothing was replaced.
    pub fn notice(&self) -> Vec<String> {
        let Some(k) = &self.kept else { return vec![] };
        let mut out = vec![];
        if !self.seen {
            let how = if k.forced { "by force" } else { "" };
            out.push(format!("flashed {how} by {} – your {} is kept", k.by, k.own).replace("  ", " "));
        }
        out.push(match (k.on_own, self.may_switch()) {
            (false, true) => format!("runs the flashed {} – switch: back to your {}", k.flashed, k.own),
            (true, true) => format!("runs your own {} – switch: to the flashed {}", k.own, k.flashed),
            (_, false) => format!("runs the flashed {} – locked: only the deck's owner can go back", k.flashed),
        });
        out
    }

    /// #471: the HUD's line for "please don't flash".
    pub fn keep_line(&self) -> Option<String> {
        self.keep.as_ref().map(|why| format!("please don't flash: {why}"))
    }

    /// The show fetched (or why not), and whether the deck carries each piece (`has`: `Err` = why not, in words): the
    /// report. Ready = nothing lacks.
    pub fn check(&mut self, show: &Pinned, doc: Result<Show, String>, has: impl Fn(&str) -> Result<(), String>) -> FlashReport {
        let lacks = lacks(show, &self.outlet, doc, has);
        self.answer(show, lacks)
    }

    /// #475: the pieces the flash said this outlet plays in `show` – what a deck fetches and checks without the document.
    /// None: the flash did not say (then the document is read, [`Deck::check`]).
    pub fn told(&self, show: &Pinned) -> Option<&[Need]> {
        let (of, mine) = self.needed.as_ref()?;
        (of == show).then(|| mine.as_deref().unwrap_or_default())
    }

    /// #475: the report from what the flash said this outlet plays – no document. None: the flash did not say.
    pub fn check_told(&mut self, show: &Pinned, has: impl Fn(&str) -> Result<(), String>) -> Option<FlashReport> {
        let (of, mine) = self.needed.clone()?;
        if &of != show {
            return None;
        }
        let lacks = match mine {
            None => vec![Lack::Outlet { show: show.clone(), outlet: self.outlet.clone() }],
            Some(needs) => needs.into_iter().filter_map(|n| has(&n.piece).err().map(|why| Lack::Piece { piece: n.piece, row: n.row, why })).collect(),
        };
        Some(self.answer(show, lacks))
    }

    fn answer(&mut self, show: &Pinned, lacks: Vec<Lack>) -> FlashReport {
        let mut report = flash::report(show, true, None);
        report.ready = Some(lacks.is_empty());
        report.lacks = (!lacks.is_empty()).then(|| lacks.iter().map(ToString::to_string).collect());
        let report = self.stamp(report);
        self.last = Some(report.clone());
        report
    }

    /// The last report, for the HUD.
    pub fn last(&self) -> Option<&FlashReport> {
        self.last.as_ref()
    }
}

/// What a deck's outlet lacks of `show` (its document, the outlet, each piece), in row order.
pub fn lacks(show: &Pinned, outlet: &str, doc: Result<Show, String>, has: impl Fn(&str) -> Result<(), String>) -> Vec<Lack> {
    let doc = match doc {
        Ok(d) => d,
        Err(why) => return vec![Lack::Show { show: show.clone(), why }],
    };
    if doc.outlet(outlet).is_none() {
        return vec![Lack::Outlet { show: show.clone(), outlet: outlet.to_string() }];
    }
    needs(&doc, outlet).into_iter().filter_map(|n| has(&n.piece).err().map(|why| Lack::Piece { piece: n.piece, row: n.row, why })).collect()
}

/// The report's line, for the HUD and the editor's list: `FLASH top@3 ready`, `FLASH top@3: 2 missing`, `FLASH refused: …`.
pub fn line(r: &FlashReport) -> String {
    let run = if r.running.id.is_empty() { "nothing".to_string() } else { format!("{}@{}", r.running.id, r.running.version) };
    if !r.accepted {
        return format!("FLASH refused: {} – runs {run}", r.why.as_deref().unwrap_or("?"));
    }
    match (r.ready, r.lacks.as_ref().map_or(0, Vec::len)) {
        (Some(true), _) => format!("FLASH {run} ready"),
        (Some(false), n) => format!("FLASH {run}: {n} missing"),
        // a conductor's report: what it runs
        (None, _) => format!("FLASH {run} taken"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP: &str = r#"{
      "id": "top", "version": 3,
      "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
      "rows": [
        { "id": "arrival", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "mirror", "of": "left" } } },
        { "id": "roar", "cells": { "left": { "mode": "values", "piece": "scene:nave", "values": { "light": 0.8 } }, "right": { "mode": "play", "piece": "character:catpope" } } },
        { "id": "exit", "cells": { "left": { "mode": "play", "piece": "scene:ghost" }, "right": { "mode": "black" } } }
      ]
    }"#;

    fn pin(id: &str, v: u64) -> protocol::Pinned {
        protocol::Pinned { id: id.into(), version: v }
    }

    fn flash(here: Option<protocol::Pinned>, hops: u64) -> Flash {
        Flash { root: pin("top", 3), shows: vec![pin("top", 3), pin("foyer-loop", 2)], lock: None, hops, here, id: None, by: None, skip: None, force: None, needs: None }
    }

    /// a flash of `top@v` with an id, by the top's operator
    fn flash_v(v: u64, id: &str) -> Flash {
        Flash { root: pin("top", v), shows: vec![pin("top", v)], lock: None, hops: 1, here: None, id: Some(id.into()), by: Some("ana".into()), skip: None, force: None, needs: None }
    }

    #[test]
    fn a_deck_left_out_refuses_and_one_not_named_takes_it_also_when_it_joins_later() {
        let mut f = flash_v(3, "f1");
        f.skip = Some(vec!["c7".into()]);
        let mut left = Deck::new("left");
        left.set_id("c7");
        let r = left.take(&f).unwrap_err();
        assert_eq!(line(&r), "FLASH refused: left out by the conductor – runs nothing");
        // a deck that joins later gets the flash from the snapshot, with a new id: not named, it takes it
        let mut late = Deck::new("left");
        late.set_id("c12");
        assert_eq!(late.take(&f).unwrap().to_string(), "top@3");
    }

    #[test]
    fn please_dont_flash_is_a_request_that_a_warned_and_accepted_flash_goes_over() {
        let mut deck = Deck::new("left");
        deck.set_id("c7");
        deck.set_keep(Some("rehearsing the foyer loop".into()));
        assert_eq!(deck.keep_line().as_deref(), Some("please don't flash: rehearsing the foyer loop"));
        let r = deck.take(&flash_v(3, "f1")).unwrap_err();
        assert_eq!(r.why.as_deref(), Some("asks not to be flashed: rehearsing the foyer loop"));
        assert_eq!(r.keep.as_deref(), Some("rehearsing the foyer loop"), "the report carries the mark to the conductor");
        let mut forced = flash_v(3, "f2");
        forced.force = Some(vec!["c7".into()]);
        assert!(deck.take(&forced).is_ok());
    }

    #[test]
    fn unsaved_configuration_needs_the_warning_too_and_is_kept() {
        let mut deck = Deck::new("left");
        deck.set_id("c7");
        deck.set_configuration(true, 3);
        assert_eq!(deck.about().unsaved, Some(true));
        assert_eq!(deck.about().mappings, Some(3));
        assert_eq!(deck.take(&flash_v(3, "f1")).unwrap_err().why.as_deref(), Some("has unsaved configuration"));
        let mut forced = flash_v(3, "f2");
        forced.force = Some(vec!["c7".into()]);
        deck.take(&forced).unwrap();
        let k = deck.kept().unwrap();
        assert!(k.forced);
        assert_eq!(deck.about().unsaved, None, "what was unsaved is the own copy now");
        assert_eq!(deck.about().mappings, Some(3), "local mappings stay");
    }

    #[test]
    fn a_flash_keeps_the_own_version_the_notice_says_so_and_the_switch_goes_both_ways() {
        let mut deck = Deck::new("left");
        deck.set_id("c7");
        deck.take(&flash_v(2, "mine")).unwrap();
        assert!(deck.notice().is_empty(), "nothing of its own was replaced by the first flash");
        deck.set_keep(Some("my version".into()));
        let mut f = flash_v(3, "f2");
        f.force = Some(vec!["c7".into()]);
        deck.take(&f).unwrap();
        assert_eq!(deck.notice(), ["flashed by force by ana – your top@2 is kept", "runs the flashed top@3 – switch: back to your top@2"]);
        assert_eq!(deck.switch(true).unwrap().to_string(), "top@2");
        let r = deck.about();
        assert_eq!((r.own, r.kept.map(|k| k.version), r.running.version), (Some(true), Some(2), 2));
        assert_eq!(deck.notice(), ["runs your own top@2 – switch: to the flashed top@3"]);
        assert_eq!(deck.switch(false).unwrap().to_string(), "top@3");
        // a later flash replaces the flashed version; the own copy stays the deck's own
        let mut g = flash_v(4, "f3");
        g.force = Some(vec!["c7".into()]);
        deck.take(&g).unwrap();
        assert_eq!(deck.kept().unwrap().own.to_string(), "top@2");
        // a rejoin hands the same flash again: nothing changes, also after switching back
        deck.switch(true).unwrap();
        assert_eq!(deck.take(&g).unwrap().to_string(), "top@2");
        assert!(deck.kept().unwrap().on_own);
    }

    #[test]
    fn the_owner_can_always_go_back_anyone_else_unless_the_flash_was_locked() {
        let mut deck = Deck::new("left");
        deck.take(&flash_v(2, "mine")).unwrap();
        let mut f = flash_v(3, "f2");
        f.lock = Some(true);
        deck.take(&f).unwrap();
        assert!(deck.may_switch(), "the owner");
        deck.set_owner(false);
        assert!(!deck.may_switch());
        assert_eq!(deck.switch(true).unwrap_err(), "the flash by ana is locked: only the deck's owner can go back");
        assert_eq!(deck.notice()[1], "runs the flashed top@3 – locked: only the deck's owner can go back");
        let mut open = flash_v(4, "f3");
        open.lock = None;
        deck.take(&open).unwrap();
        assert!(deck.may_switch(), "not locked: anyone at the deck");
        assert!(Deck::new("x").switch(true).is_err(), "nothing kept");
    }

    #[test]
    fn an_outlet_needs_each_piece_once_and_a_mirror_plays_what_its_outlet_plays() {
        let show = read(TOP).unwrap();
        let n = needs(&show, "right");
        assert_eq!(n, [Need { piece: "scene:nave".into(), row: "arrival".into() }, Need { piece: "character:catpope".into(), row: "roar".into() }]);
        assert_eq!(needs(&show, "left").len(), 2, "values names the same piece");
        assert!(needs(&show, "lights").is_empty(), "a device plays what a row sends");
    }

    #[test]
    fn a_deck_takes_the_show_of_its_session_and_says_what_it_lacks() {
        let mut deck = Deck::new("left");
        let show = deck.take(&flash(Some(pin("top", 3)), 1)).unwrap();
        assert_eq!(show.to_string(), "top@3");
        let r = deck.check(&show, read(TOP), |p| if p == "scene:ghost" { Err("scene:ghost is not on this deck".into()) } else { Ok(()) });
        assert!(r.accepted);
        assert_eq!(r.ready, Some(false));
        assert_eq!(r.lacks.as_deref(), Some(&["row exit: scene:ghost is not on this deck".to_string()][..]));
        assert_eq!(line(&r), "FLASH top@3: 1 missing");
        let r = deck.check(&show, read(TOP), |_| Ok(()));
        assert_eq!((r.ready, line(&r)), (Some(true), "FLASH top@3 ready".to_string()));
        assert_eq!(r.outlet.as_deref(), Some("left"));
    }

    #[test]
    fn a_missing_show_or_outlet_is_one_reason() {
        let p = Pinned { id: "top".into(), version: 3 };
        assert_eq!(lacks(&p, "left", Err("offline".into()), |_| Ok(()))[0].to_string(), "top@3: the show is not here (offline)");
        assert_eq!(lacks(&p, "foyer", read(TOP), |_| Ok(()))[0].to_string(), "top@3 has no outlet 'foyer': no cell will come");
        let broken = Lack::Piece { piece: "scene:nave".into(), row: "arrival".into(), why: "a node 'x' of an unknown kind".into() };
        assert_eq!(broken.to_string(), "row arrival: scene:nave: a node 'x' of an unknown kind");
    }

    #[test]
    fn without_here_the_root_and_a_nearer_hand_refuses() {
        let mut deck = Deck::new("left");
        assert_eq!(deck.take(&flash(None, 0)).unwrap().to_string(), "top@3");
        let r = deck.take(&flash(None, 2)).unwrap_err();
        assert!(!r.accepted);
        assert_eq!(line(&r), "FLASH refused: assigned by a nearer hand – runs top@3");
        assert_eq!(deck.last(), Some(&*r));
    }

    /// #475: what a conductor sends with a flash – every outlet, the pieces its cells name
    #[test]
    fn a_flash_says_what_each_outlet_plays() {
        let needs = flash_needs(&read(TOP).unwrap());
        let pieces = |outlet: &str| needs[outlet].iter().map(|n| format!("{} ({})", n.piece, n.row)).collect::<Vec<_>>();
        assert_eq!(pieces("left"), ["scene:nave (arrival)", "scene:ghost (exit)"]);
        assert_eq!(pieces("right"), ["scene:nave (arrival)", "character:catpope (roar)"], "a mirror plays what its outlet plays");
        assert!(pieces("lights").is_empty(), "a device outlet is listed with none");
        // a deck takes it: its outlet's pieces without the document; an outlet the show lacks gets no cell
        let mut f = flash(Some(pin("top", 3)), 1);
        f.needs = Some(needs);
        let mut deck = Deck::new("left");
        let show = deck.take(&f).unwrap();
        assert_eq!(deck.told(&show).map(<[Need]>::len), Some(2));
        let r = deck.check_told(&show, |p| if p == "scene:ghost" { Err("scene:ghost is not on this deck".into()) } else { Ok(()) }).unwrap();
        assert_eq!((r.ready, r.lacks.as_deref()), (Some(false), Some(&["row exit: scene:ghost is not on this deck".to_string()][..])));
        let mut foyer = Deck::new("foyer");
        let show = foyer.take(&f).unwrap();
        assert_eq!(foyer.check_told(&show, |_| Ok(())).unwrap().lacks.as_deref(), Some(&["top@3 has no outlet 'foyer': no cell will come".to_string()][..]));
        // a flash that does not say it: the document is read as before
        let mut old = Deck::new("left");
        let show = old.take(&flash(Some(pin("top", 3)), 1)).unwrap();
        assert!(old.told(&show).is_none() && old.check_told(&show, |_| Ok(())).is_none());
    }

    #[test]
    fn the_api_wraps_a_document_and_the_cache_keeps_it_bare() {
        let wrapped = format!(r#"{{ "content": {TOP}, "version": {{ "version": 3 }} }}"#);
        assert_eq!(read(&wrapped).unwrap().id, "top");
        // a pinned version, as a flash fetches it (server ShowsController::get with `v`)
        assert_eq!(read(&format!(r#"{{ "status": "success", "show": {TOP} }}"#)).unwrap().version, 3);
        assert_eq!(read(TOP).unwrap().version, 3);
        assert!(read("{}").is_err());
        assert_eq!(kept_as(&Pinned { id: "top".into(), version: 3 }), "shows/top@3.json");
    }
}
