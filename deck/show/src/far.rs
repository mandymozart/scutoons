//! Far world nodes across a row's outlets (#568, SHOW.md §6 "cells of the same row: while both ends stay on stage"): which
//! piece plays on each stage outlet when a row fires (a kept cell ↑ and a question count as this row: the piece above plays
//! on), and for each far world node of those pieces where its holder plays – the piece itself, another outlet of the row,
//! or nowhere in it. The pieces' reach is deck-space's (holders.rs: what a piece holds, its far nodes); the editor's loader
//! hands it in and draws what this says: the wire between two cells, Build's tube off the stage.
//!
//! A holder is matched by its place (`character:bells`); a `deck:` place that names one of the show's outlets narrows it
//! to that outlet (a deck plays one outlet), any other deck name is the session's and does not narrow. Whether the leaf
//! names a real output of the holder is the deck's (far.rs `offers`) and the validator's, not the row's.
use crate::document::{Cell, OutletKind, Show};
use deck_space::holders::Reach;

/// Where a far node's holder plays in the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    /// the piece that listens holds it itself (a scene with both)
    Here,
    /// the piece on this outlet of the row holds it
    Outlet(String),
    /// no piece of the row holds it: off the stage, its address says where
    Away,
}

/// One far world node of a piece on an outlet of a row, and where its holder plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub row: String,
    /// the outlet whose piece listens
    pub from: String,
    /// the far world node (its compiled id in the piece)
    pub node: String,
    pub address: String,
    pub holder: Holder,
}

impl Link {
    /// the wire's words: `far_in ← right · /deck:right/character:bells/far:in.events`
    pub fn words(&self) -> String {
        match &self.holder {
            Holder::Here => format!("{} ← this piece · {}", self.node, self.address),
            Holder::Outlet(o) => format!("{} ← {o} · {}", self.node, self.address),
            Holder::Away => format!("{} ← not in this row · {}", self.node, self.address),
        }
    }
}

/// The piece on each stage outlet when row `row` fires after the rows above it, in the outlets' order; an outlet whose
/// path plays none is left out.
pub fn on_row(show: &Show, row: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for o in show.outlets.iter().filter(|o| o.kind == OutletKind::Stage) {
        for r in show.rows[..=row.min(show.rows.len().saturating_sub(1))].iter().rev() {
            match r.cells.get(&o.id) {
                None | Some(Cell::Keep | Cell::Ask { .. }) => continue,
                Some(Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. }) => out.push((o.id.clone(), p.clone())),
                Some(_) => {}
            }
            break;
        }
    }
    out
}

/// Every far node of the pieces on row `row` (by id) and where its holder plays. `reach` is a piece's reach, none while it
/// is not known (its pieces' nodes are left out until it is).
pub fn links(show: &Show, row: &str, reach: impl Fn(&str) -> Option<Reach>) -> Vec<Link> {
    let Some(i) = show.rows.iter().position(|r| r.id == row) else { return Vec::new() };
    let pieces = on_row(show, i);
    let known: Vec<(&str, Option<Reach>)> = pieces.iter().map(|(o, p)| (o.as_str(), reach(p))).collect();
    let mut out = Vec::new();
    for (from, r) in &known {
        let Some(r) = r else { continue };
        for f in &r.far {
            let deck = f.address.places.iter().find(|p| p.kind == "deck").map(|p| p.id.as_str()).filter(|d| show.outlet(d).is_some());
            let holder = match f.holder() {
                None => Holder::Away,
                Some(h) if r.holds(h) && deck.is_none_or(|d| d == *from) => Holder::Here,
                Some(h) => known
                    .iter()
                    .find(|(o, x)| o != from && deck.is_none_or(|d| d == *o) && x.as_ref().is_some_and(|x| x.holds(h)))
                    .map_or(Holder::Away, |(o, _)| Holder::Outlet(o.to_string())),
            };
            out.push(Link { row: row.to_string(), from: from.to_string(), node: f.node.clone(), address: f.address.to_string(), holder });
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::document::parse;
    use deck_space::piece::Found;
    use deck_space::rig::address::Place;

    pub(crate) const SHOW: &str = include_str!("../fixtures/far-outlets.show.json");

    /// the fixture's pieces as the editor's loader reads them (the rigs beside the show)
    pub(crate) fn find(place: &Place) -> Option<Found> {
        match place.id.as_str() {
            "keys" => Some(Found::Rig(include_str!("../fixtures/rigs/keys.toml").into())),
            "bells" => Some(Found::Rig(include_str!("../fixtures/rigs/bells.toml").into())),
            _ => None,
        }
    }

    pub(crate) fn reach(piece: &str) -> Option<Reach> {
        deck_space::holders::read(piece, find).ok().map(|r| r.reach)
    }

    #[test]
    fn the_fixture_validates() {
        let show = parse(SHOW).unwrap();
        assert_eq!(crate::validate(&show), vec![], "the far-outlets show has no problems");
        assert!(crate::values::warnings(&show).is_empty());
    }

    /// a kept cell and a question play on the piece above; black ends it
    #[test]
    fn the_pieces_of_a_row_follow_the_path() {
        let show = parse(SHOW).unwrap();
        assert_eq!(on_row(&show, 0), [("left".to_string(), "character:keys".to_string()), ("middle".into(), "item:incense_burner".into())]);
        assert_eq!(on_row(&show, 1), [("left".to_string(), "character:keys".to_string()), ("middle".into(), "item:incense_burner".into()), ("right".into(), "character:bells".into())]);
        assert_eq!(on_row(&show, 2).iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>(), ["right"]);
    }

    /// the keys listen to the bells on `right`, the bells to the keys wherever they play: connected while both are on
    #[test]
    fn far_nodes_are_wired_to_the_outlet_their_holder_plays_on() {
        let show = parse(SHOW).unwrap();
        let alone = links(&show, "keys-alone", reach);
        assert_eq!(alone.len(), 1);
        assert_eq!((alone[0].from.as_str(), alone[0].node.as_str(), &alone[0].holder), ("left", "far_in", &Holder::Away));
        let both = links(&show, "together", reach);
        let pairs: Vec<(&str, &Holder)> = both.iter().map(|l| (l.from.as_str(), &l.holder)).collect();
        assert_eq!(pairs, [("left", &Holder::Outlet("right".into())), ("right", &Holder::Outlet("left".into()))]);
        assert_eq!(both[0].words(), "far_in ← right · /deck:right/character:bells/far:in.events");
        let after = links(&show, "bells-alone", reach);
        assert_eq!(after.iter().map(|l| &l.holder).collect::<Vec<_>>(), [&Holder::Away]);
        // a piece not known yet: its far nodes wait, and nobody holds what it would
        let half = links(&show, "together", |p| (p != "character:bells").then(|| reach(p)).flatten());
        assert_eq!(half.iter().map(|l| (l.from.as_str(), &l.holder)).collect::<Vec<_>>(), [("left", &Holder::Away)]);
        assert!(links(&show, "nope", reach).is_empty());
    }

    /// a `deck:` place that names an outlet narrows the holder to it
    #[test]
    fn a_deck_place_names_the_outlet() {
        let mut show = parse(SHOW).unwrap();
        let row = show.rows.iter_mut().find(|r| r.id == "together").unwrap();
        let bells = row.cells.remove("right").unwrap();
        row.cells.insert("middle".into(), bells);
        let l = links(&show, "together", reach);
        assert_eq!(l.iter().find(|l| l.from == "left").unwrap().holder, Holder::Away, "the bells on middle are not /deck:right");
        assert_eq!(l.iter().find(|l| l.from == "middle").unwrap().holder, Holder::Outlet("left".into()));
    }
}
