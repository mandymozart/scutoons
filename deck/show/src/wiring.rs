//! Cells wired to cells by their ports, within one row: a cell's ports come from what it holds and carry a signal
//! (deck-kit signal.rs, the one table); a wire runs from one cell's outlet into another's inlet of a signal it feeds. The
//! node view and the rows band draw the same ports and wires. A busy row is shown simplified: the wires between two cells
//! hang as one bundle with their count, every wire shows on its own only for the cell in focus.
use crate::document::{Cell, OutletKind, Row, Show};
use deck_kit::base::graph::Pin;
use deck_kit::base::signal::Signal;
use serde::{Deserialize, Serialize};

/// One wire of a row: from `from`'s outlet `out` into `to`'s inlet `in` (column ids, port names as `ports` gives them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellWire {
    pub from: String,
    pub out: String,
    pub to: String,
    #[serde(rename = "in")]
    pub into: String,
}

impl CellWire {
    pub fn new(from: &str, out: &str, to: &str, into: &str) -> CellWire {
        CellWire { from: from.into(), out: out.into(), to: to.into(), into: into.into() }
    }

    pub fn words(&self) -> String {
        format!("{} {} → {} {}", self.from, self.out, self.to, self.into)
    }
}

pub const FIRE: &str = "fire";
pub const ENDS: &str = "ends";
pub const MIRRORS: &str = "mirrors";

/// A cell's ports: its inlets and its outlets. A kept cell has none; a far node's inlet is the node view's (nodes.rs).
pub fn ports(show: &Show, column: &str, cell: &Cell) -> (Vec<Pin>, Vec<Pin>) {
    let mut ins = Vec::new();
    if matches!(cell, Cell::Keep) {
        return (ins, Vec::new());
    }
    ins.push(Pin::new(FIRE, Signal::Event));
    if let Cell::Play { values, .. } | Cell::Values { values, .. } = cell {
        ins.extend(values.iter().map(|(k, v)| Pin::new(k.clone(), Signal::Control).valued(v.as_str().map_or_else(|| v.to_string(), String::from))));
    }
    if let Cell::Mirror { .. } = cell {
        ins.push(Pin::new(MIRRORS, Signal::Scene));
    }
    let Some(kind) = show.outlet(column).map(|o| o.kind) else { return (ins, Vec::new()) };
    let mut outs = match (cell, kind) {
        (Cell::Ask { .. }, _) => vec![Pin::new("answer", Signal::Vote)],
        (Cell::Black | Cell::Go { .. } | Cell::Free, _) => Vec::new(),
        (_, OutletKind::Device) => vec![Pin::new("sends", Signal::Address)],
        (_, OutletKind::Stage) => vec![Pin::new("plays", Signal::Scene)],
    };
    if matches!(cell, Cell::Play { .. } | Cell::Look { .. } | Cell::Ask { .. }) {
        outs.push(Pin::new(ENDS, Signal::Event));
    }
    (ins, outs)
}

/// whether an inlet takes a wire drawn by hand: a mirror's is its `of`, a far node's comes from far
pub fn wireable(pin: &Pin) -> bool {
    matches!(pin.signal, Signal::Event | Signal::Control)
}

fn ports_of(show: &Show, row: &Row, column: &str) -> (Vec<Pin>, Vec<Pin>) {
    match row.cells.get(column) {
        Some(cell) => ports(show, column, cell),
        None => (Vec::new(), Vec::new()),
    }
}

/// A wire of a row as drawn: which of its ends' ports it joins and the signal it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub wire: CellWire,
    pub out_at: usize,
    pub in_at: usize,
    pub signal: Signal,
}

/// The row's wires whose ends stand (`check` passes), placed on their ports.
pub fn placed(show: &Show, row: &Row) -> Vec<Placed> {
    row.wires
        .iter()
        .filter_map(|w| {
            let (_, outs) = ports_of(show, row, &w.from);
            let (ins, _) = ports_of(show, row, &w.to);
            let out_at = outs.iter().position(|p| p.label == w.out)?;
            let in_at = ins.iter().position(|p| p.label == w.into)?;
            Some(Placed { wire: w.clone(), out_at, in_at, signal: outs[out_at].signal })
        })
        .collect()
}

/// Why a wire of `row` cannot stand, if it cannot.
pub fn check(show: &Show, row: &Row, w: &CellWire) -> Option<String> {
    if w.from == w.to {
        return Some(format!("{}: a cell is not wired to itself", w.words()));
    }
    let (_, outs) = ports_of(show, row, &w.from);
    let (ins, _) = ports_of(show, row, &w.to);
    let Some(out) = outs.iter().find(|p| p.label == w.out) else {
        return Some(format!("{}: {} has no outlet '{}' in this row", w.words(), w.from, w.out));
    };
    let Some(into) = ins.iter().find(|p| p.label == w.into) else {
        return Some(format!("{}: {} has no inlet '{}' in this row", w.words(), w.to, w.into));
    };
    if !wireable(into) {
        return Some(format!("{}: '{}' is not wired by hand", w.words(), w.into));
    }
    if !out.signal.feeds(into.signal) {
        return Some(format!("{}: {} does not feed {}", w.words(), out.signal.word(), into.signal.word()));
    }
    if row.wires.iter().filter(|x| *x == w).count() > 1 {
        return Some(format!("{}: wired twice", w.words()));
    }
    None
}

/// Every wire whose ends no longer stand (a cell changed its mode, kept, lost a value) goes; the words of what went.
pub fn prune(show: &mut Show) -> Vec<String> {
    let mut gone = Vec::new();
    for i in 0..show.rows.len() {
        let row = show.rows[i].clone();
        let mut seen: Vec<&CellWire> = Vec::new();
        let keep: Vec<bool> = row
            .wires
            .iter()
            .map(|w| {
                let twice = seen.contains(&w);
                seen.push(w);
                let ok = !twice && check(show, &Row { wires: vec![w.clone()], ..row.clone() }, w).is_none();
                if !ok {
                    gone.push(w.words());
                }
                ok
            })
            .collect();
        let mut k = keep.iter();
        show.rows[i].wires.retain(|_| *k.next().unwrap_or(&true));
    }
    gone
}

/// How many wires make a row busy: from here on a pair of cells' wires hang as one bundle.
pub const BUSY: usize = 3;

/// What the band draws of a row's wires.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// one wire on its typed ports (its index in `placed`)
    One(usize),
    /// the wires between two cells (column ids, in the row's order) as one, with their count; one signal or mixed
    Bundle { a: String, b: String, wires: Vec<usize>, signal: Option<Signal> },
}

/// The simplified view of a row's placed wires: in a busy row (`BUSY` wires or more) the wires between two cells are one
/// bundle; every wire touching `focus` (the selected or hovered cell) shows on its own. `order` = the columns in the
/// band's order.
pub fn simplified(placed: &[Placed], order: &[String], focus: &[&str]) -> Vec<Shown> {
    let mut out = Vec::new();
    let busy = placed.len() >= BUSY;
    let at = |c: &str| order.iter().position(|x| x == c).unwrap_or(usize::MAX);
    let mut bundles: Vec<(String, String, Vec<usize>)> = Vec::new();
    for (i, p) in placed.iter().enumerate() {
        let w = &p.wire;
        if !busy || focus.contains(&w.from.as_str()) || focus.contains(&w.to.as_str()) {
            out.push(Shown::One(i));
            continue;
        }
        let (a, b) = if at(&w.from) <= at(&w.to) { (&w.from, &w.to) } else { (&w.to, &w.from) };
        match bundles.iter_mut().find(|(x, y, _)| x == a && y == b) {
            Some((_, _, v)) => v.push(i),
            None => bundles.push((a.clone(), b.clone(), vec![i])),
        }
    }
    for (a, b, wires) in bundles {
        if let [one] = wires[..] {
            out.push(Shown::One(one));
            continue;
        }
        let first = placed[wires[0]].signal;
        let signal = wires.iter().all(|i| placed[*i].signal == first).then_some(first);
        out.push(Shown::Bundle { a, b, wires, signal });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;

    const SHOW: &str = r#"{ "id": "w", "version": 1, "toon": "catpope",
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "lights", "kind": "device" }],
        "rows": [{ "id": "one", "cells": {
            "wall": { "mode": "play", "piece": "character:keys", "values": { "speed": 0.5 } },
            "floor": { "mode": "play", "piece": "character:bells", "values": { "glow": 1.0, "speed": 0.2 } } } }] }"#;

    fn show() -> Show {
        parse(SHOW).unwrap()
    }

    #[test]
    fn a_cell_has_typed_ports_and_a_kept_cell_none() {
        let s = show();
        let (ins, outs) = ports(&s, "floor", &s.rows[0].cells["floor"]);
        assert_eq!(ins.iter().map(|p| (p.label.as_str(), p.signal)).collect::<Vec<_>>(), [("fire", Signal::Event), ("glow", Signal::Control), ("speed", Signal::Control)]);
        assert_eq!(outs.iter().map(|p| (p.label.as_str(), p.signal)).collect::<Vec<_>>(), [("plays", Signal::Scene), ("ends", Signal::Event)]);
        let (_, outs) = ports(&s, "lights", &Cell::Look { look: "warm".into(), fade: None });
        assert_eq!(outs.iter().map(|p| p.signal).collect::<Vec<_>>(), [Signal::Address, Signal::Event]);
        assert_eq!(ports(&s, "wall", &Cell::Keep), (vec![], vec![]));
    }

    #[test]
    fn a_wire_stands_only_between_ports_whose_signals_fit() {
        let s = show();
        let row = &s.rows[0];
        assert_eq!(check(&s, row, &CellWire::new("wall", "ends", "floor", "fire")), None);
        assert!(check(&s, row, &CellWire::new("wall", "ends", "wall", "fire")).unwrap().contains("itself"));
        assert!(check(&s, row, &CellWire::new("wall", "plays", "floor", "fire")).unwrap().contains("does not feed"));
        assert!(check(&s, row, &CellWire::new("wall", "ends", "floor", "nope")).unwrap().contains("no inlet"));
        assert!(check(&s, row, &CellWire::new("lights", "ends", "floor", "fire")).unwrap().contains("no outlet"), "a column without a cell in the row");
    }

    #[test]
    fn wires_whose_ends_went_are_pruned_and_a_twin_too() {
        let mut s = show();
        s.rows[0].wires = vec![CellWire::new("wall", "ends", "floor", "glow"), CellWire::new("wall", "ends", "floor", "fire"), CellWire::new("wall", "ends", "floor", "fire")];
        assert!(prune(&mut s).len() == 2, "glow is a control, not an event, and the twin goes");
        assert_eq!(s.rows[0].wires, [CellWire::new("wall", "ends", "floor", "fire")]);
        s.rows[0].cells.insert("floor".into(), Cell::Keep);
        assert_eq!(prune(&mut s), ["wall ends → floor fire"]);
        assert!(s.rows[0].wires.is_empty());
    }

    #[test]
    fn the_node_view_draws_the_same_ports_and_wire() {
        let mut s = show();
        s.rows[0].wires.push(CellWire::new("wall", "ends", "floor", "speed"));
        assert!(check(&s, &s.rows[0], &s.rows[0].wires[0]).is_some(), "an event feeds no value");
        s.rows[0].wires[0] = CellWire::new("wall", "ends", "floor", "fire");
        let g = crate::nodes::row_graph(&s, 0, &[]);
        let (wall, floor) = (g.node("wall").unwrap(), g.node("floor").unwrap());
        let l = g.links.iter().find(|l| l.from.0 == wall && l.to.0 == floor).expect("the wire in the node view");
        assert_eq!((g.nodes[wall].outs[l.from.1].label.as_str(), g.nodes[floor].ins[l.to.1].label.as_str(), l.signal), ("ends", "fire", Signal::Event));
        assert_eq!(g.nodes[floor].ins.len(), ports(&s, "floor", &s.rows[0].cells["floor"]).0.len());
    }

    #[test]
    fn a_wire_round_trips_through_the_document() {
        let mut s = show();
        s.rows[0].wires.push(CellWire::new("wall", "ends", "floor", "fire"));
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""wires":[{"from":"wall","out":"ends","to":"floor","in":"fire"}]"#), "{text}");
        assert_eq!(parse(&text).unwrap(), s);
    }

    #[test]
    fn a_busy_row_bundles_the_wires_between_two_cells_and_the_focus_shows_its_own() {
        let p = |from: &str, to: &str, signal: Signal| Placed { wire: CellWire::new(from, "o", to, "i"), out_at: 0, in_at: 0, signal };
        let order: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let quiet = [p("a", "b", Signal::Event), p("b", "a", Signal::Event)];
        assert_eq!(simplified(&quiet, &order, &[]), [Shown::One(0), Shown::One(1)], "a quiet row shows every wire");
        let busy = [p("a", "b", Signal::Event), p("b", "a", Signal::Event), p("a", "c", Signal::Control), p("a", "b", Signal::Control)];
        let s = simplified(&busy, &order, &[]);
        assert_eq!(s, [Shown::Bundle { a: "a".into(), b: "b".into(), wires: vec![0, 1, 3], signal: None }, Shown::One(2)]);
        let s = simplified(&busy[..3], &order, &[]);
        assert!(matches!(&s[0], Shown::Bundle { signal: Some(Signal::Event), wires, .. } if wires.len() == 2), "one signal: its colour");
        assert_eq!(simplified(&busy, &order, &["b"]).iter().filter(|x| matches!(x, Shown::One(_))).count(), 4, "b's wires on their own, a → c alone anyway");
    }
}
