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

/// The file a deck keeps a flashed show in, under its cache: one per exact version (a version never changes).
pub fn kept_as(p: &Pinned) -> String {
    format!("shows/{}@{}.json", p.id, p.version)
}

/// A show document as the API gives it (`GET /api/shows/<id>?v=<n>`: wrapped in `content`) or as it is kept (bare).
pub fn read(text: &str) -> Result<Show, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let doc = match v.get("content") {
        Some(c) if c.is_object() => c.clone(),
        _ => v,
    };
    document::parse(&doc.to_string()).map_err(|e| e.to_string())
}

/// The flash side of one deck's outlet.
#[derive(Debug, Clone)]
pub struct Deck {
    outlet: String,
    standing: Option<Assignment>,
    /// the show this outlet plays from, as the last flash it took assigned it
    running: Option<Pinned>,
    last: Option<FlashReport>,
}

impl Deck {
    pub fn new(outlet: &str) -> Deck {
        Deck { outlet: outlet.to_string(), standing: None, running: None, last: None }
    }

    fn running(&self) -> Pinned {
        self.running.clone().unwrap_or(Pinned { id: String::new(), version: 0 })
    }

    /// A flash arrives. `Ok` = taken: the show to fetch (or read from what was kept) and then [`Deck::check`]; `Err` = the
    /// report of a refusal, to send at once. The show is the flash's `here` (the conductor of this deck's session), else its
    /// root.
    pub fn take(&mut self, flash: &Flash) -> Result<Pinned, FlashReport> {
        let running = self.running();
        let mut report = flash::take(&mut self.standing, flash, &running, None);
        report.outlet = Some(self.outlet.clone());
        if !report.accepted {
            self.last = Some(report.clone());
            return Err(report);
        }
        let here = flash.here.as_ref().unwrap_or(&flash.root);
        let show = flash::pinned(here);
        self.running = Some(show.clone());
        Ok(show)
    }

    /// The show fetched (or why not), and whether the deck carries each piece (`has`: `Err` = why not, in words): the
    /// report. Ready = nothing lacks.
    pub fn check(&mut self, show: &Pinned, doc: Result<Show, String>, has: impl Fn(&str) -> Result<(), String>) -> FlashReport {
        let lacks = lacks(show, &self.outlet, doc, has);
        let report = FlashReport {
            running: flash::wire(show),
            accepted: true,
            why: None,
            outlet: Some(self.outlet.clone()),
            ready: Some(lacks.is_empty()),
            lacks: (!lacks.is_empty()).then(|| lacks.iter().map(ToString::to_string).collect()),
        };
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
        Flash { root: pin("top", 3), shows: vec![pin("top", 3), pin("foyer-loop", 2)], lock: None, hops, here }
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
        assert_eq!(deck.last(), Some(&r));
    }

    #[test]
    fn the_api_wraps_a_document_and_the_cache_keeps_it_bare() {
        let wrapped = format!(r#"{{ "content": {TOP}, "version": {{ "version": 3 }} }}"#);
        assert_eq!(read(&wrapped).unwrap().id, "top");
        assert_eq!(read(TOP).unwrap().version, 3);
        assert!(read("{}").is_err());
        assert_eq!(kept_as(&Pinned { id: "top".into(), version: 3 }), "shows/top@3.json");
    }
}
