//! The wiring gestures as row edits, for the rows band and the node view alike: a press on a port takes a wire in the
//! hand (new from an outlet, lifted off a wired inlet), the hand's wire lands on an inlet or says why not, letting go
//! wires, moves or removes it, a cut takes the crossed wires, a cell dropped on a wire stands between its ends. Every
//! change is `rows::Edit::Wire` / `Unwire` (several as one step: `rows::edit_all`); a view only says which port, wire
//! or node the pointer is on – the node view by its place in the graph it shows (`*_at`).
use crate::document::{Row, Show};
use crate::rows::Edit;
use crate::wiring::{self, CellWire};
use deck_kit::base::graph::Graph;
use deck_kit::base::signal::Signal;

/// a wire in the hand: from `from`'s outlet `out` (its signal), new or lifted off the inlet of `lifted`
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub from: String,
    pub out: String,
    pub signal: Signal,
    pub lifted: Option<CellWire>,
}

/// A press on `column`'s port `pin`: an outlet gives a new wire, a wired inlet its wire (the last made), lifted; else
/// why nothing is taken.
pub fn take(show: &Show, row: &Row, column: &str, out: bool, pin: &str) -> Result<Held, String> {
    let (ins, outs) = wiring::ports_of(show, row, column);
    if out {
        let p = outs.iter().find(|p| p.label == pin).ok_or_else(|| format!("{column} {pin}: no outlet a wire is drawn from"))?;
        return Ok(Held { from: column.into(), out: p.label.clone(), signal: p.signal, lifted: None });
    }
    let p = ins.iter().find(|p| p.label == pin).ok_or_else(|| format!("{column} {pin}: not wired by hand"))?;
    let w = row.wires.iter().rev().find(|w| w.to == column && w.into == pin).ok_or_else(|| format!("{column} {pin}: drag from an outlet into it to wire it"))?;
    let signal = wiring::ports_of(show, row, &w.from).1.iter().find(|o| o.label == w.out).map_or(p.signal, |o| o.signal);
    Ok(Held { from: w.from.clone(), out: w.out.clone(), signal, lifted: Some(w.clone()) })
}

/// The wire `held` makes into `column`'s inlet `pin`, or why it cannot stand there (`wiring::check`'s words).
pub fn lands(show: &Show, row: &Row, held: &Held, column: &str, pin: &str) -> Result<CellWire, String> {
    let wire = CellWire::new(&held.from, &held.out, column, pin);
    if held.lifted.as_ref() == Some(&wire) {
        return Ok(wire);
    }
    let mut r = row.clone();
    r.wires.retain(|w| Some(w) != held.lifted.as_ref());
    if r.wires.contains(&wire) {
        return Err(format!("{} is wired already", wire.words()));
    }
    r.wires.push(wire.clone());
    match wiring::check(show, &r, &wire) {
        Some(why) => Err(why),
        None => Ok(wire),
    }
}

/// The hand lets go: over an inlet (`into`: column, inlet) the wire is made there, a lifted one moved; a lifted wire
/// let go on nothing goes. None: nothing changes.
pub fn let_go(row: &str, held: &Held, into: Option<(&str, &str)>) -> Option<Edit> {
    match (into, &held.lifted) {
        (Some((column, pin)), lifted) => {
            let wire = CellWire::new(&held.from, &held.out, column, pin);
            (lifted.as_ref() != Some(&wire)).then(|| Edit::Wire { row: row.into(), wire, replaces: lifted.clone() })
        }
        (None, Some(lifted)) => Some(Edit::Unwire { row: row.into(), wire: lifted.clone() }),
        (None, None) => None,
    }
}

/// A cut across `crossed`: the row's wires among them go; only wires the row makes itself were crossed: why not.
pub fn cut(row: &Row, crossed: &[CellWire]) -> Result<Vec<Edit>, String> {
    let mut gone: Vec<&CellWire> = Vec::new();
    for w in crossed.iter().filter(|w| row.wires.contains(w)) {
        if !gone.contains(&w) {
            gone.push(w);
        }
    }
    if gone.is_empty() && !crossed.is_empty() {
        return Err("nothing cut: only a wire drawn by hand is cut – the row makes the others itself".into());
    }
    Ok(gone.into_iter().map(|w| Edit::Unwire { row: row.id.clone(), wire: w.clone() }).collect())
}

/// The cell of `column` dropped on the wire `on`: the wire goes, its outlet runs into the cell's first inlet it feeds
/// and the cell's first outlet that feeds the wire's inlet runs on into it; or why it does not fit.
pub fn insert(show: &Show, row: &Row, column: &str, on: &CellWire) -> Result<Vec<Edit>, String> {
    if !row.wires.contains(on) {
        return Err(format!("{column} is not put on {}: only on a wire drawn by hand", on.words()));
    }
    if column == on.from || column == on.to {
        return Err(format!("{column} is an end of {}", on.words()));
    }
    let signal_of = |pins: Vec<deck_kit::base::graph::Pin>, label: &str| pins.iter().find(|p| p.label == label).map(|p| p.signal);
    let carried = signal_of(wiring::ports_of(show, row, &on.from).1, &on.out);
    let wanted = signal_of(wiring::ports_of(show, row, &on.to).0, &on.into);
    let (Some(carried), Some(wanted)) = (carried, wanted) else { return Err(format!("{}: its ends do not stand", on.words())) };
    let (ins, outs) = wiring::ports_of(show, row, column);
    let held = Held { from: on.from.clone(), out: on.out.clone(), signal: carried, lifted: None };
    let into = ins.iter().find_map(|p| lands(show, row, &held, column, &p.label).ok()).ok_or_else(|| format!("{column} has no free inlet {} feeds", carried.word()))?;
    let free = |o: &&deck_kit::base::graph::Pin| o.signal.feeds(wanted) && !row.wires.contains(&CellWire::new(column, &o.label, &on.to, &on.into));
    let out = outs.iter().find(free).ok_or_else(|| format!("{column} has no outlet that feeds {}", wanted.word()))?;
    let on_from = CellWire::new(column, &out.label, &on.to, &on.into);
    let r = row.id.clone();
    Ok(vec![Edit::Unwire { row: r.clone(), wire: on.clone() }, Edit::Wire { row: r.clone(), wire: into, replaces: None }, Edit::Wire { row: r, wire: on_from, replaces: None }])
}

/// a pin of the graph shown: its node, whether an outlet, which of them
pub type At = (usize, bool, usize);

/// the cell port behind a pin of the row's graph: its column and name; why not, when the node is no cell of the row
fn port<'a>(row: &Row, g: &'a Graph, (node, out, pin): At) -> Result<(&'a str, &'a str), String> {
    let n = g.nodes.get(node).ok_or("no such node")?;
    let p = if out { n.outs.get(pin) } else { n.ins.get(pin) }.ok_or("no such pin")?;
    if !row.cells.contains_key(&n.id) {
        return Err(format!("{} {}: not a cell's port – a wire is drawn between cells of the row", n.title, p.label));
    }
    Ok((&n.id, &p.label))
}

/// [`take`] by a pin of the graph shown
pub fn take_at(show: &Show, row: &Row, g: &Graph, at: At) -> Result<Held, String> {
    let (column, pin) = port(row, g, at)?;
    take(show, row, column, at.1, pin)
}

/// [`lands`] by a pin of the graph shown
pub fn lands_at(show: &Show, row: &Row, g: &Graph, held: &Held, at: At) -> Result<CellWire, String> {
    let (column, pin) = port(row, g, at)?;
    if at.1 {
        return Err(format!("{column} {pin}: a wire ends in an inlet"));
    }
    lands(show, row, held, column, pin)
}

/// the inlets of the graph shown (node, inlet) the wire in the hand may go into
pub fn targets(show: &Show, row: &Row, g: &Graph, held: &Held) -> Vec<(usize, usize)> {
    let pins = g.nodes.iter().enumerate().flat_map(|(i, n)| (0..n.ins.len()).map(move |k| (i, k)));
    pins.filter(|&(i, k)| lands_at(show, row, g, held, (i, false, k)).is_ok()).collect()
}

/// the wire a link of the graph shown would be as a cell wire (its ends by node and pin name); the row's own when it
/// is among `row.wires`
pub fn link_wire(g: &Graph, link: usize) -> Option<CellWire> {
    let l = g.links.get(link)?;
    let (a, b) = (g.nodes.get(l.from.0)?, g.nodes.get(l.to.0)?);
    Some(CellWire::new(&a.id, &a.outs.get(l.from.1)?.label, &b.id, &b.ins.get(l.to.1)?.label))
}

/// [`let_go`] over a pin of the graph shown (none: let go on nothing); an outlet under the pointer is nothing
pub fn let_go_at(row: &Row, g: &Graph, held: &Held, over: Option<At>) -> Result<Option<Edit>, String> {
    let into = match over.filter(|at| !at.1) {
        Some(at) => Some(port(row, g, at)?),
        None => None,
    };
    Ok(let_go(&row.id, held, into))
}

/// [`cut`] by the links of the graph shown the cut crossed
pub fn cut_at(row: &Row, g: &Graph, links: &[usize]) -> Result<Vec<Edit>, String> {
    let crossed: Vec<CellWire> = links.iter().filter_map(|&i| link_wire(g, i)).collect();
    cut(row, &crossed)
}

/// [`insert`] by the node dropped and the link it was dropped on
pub fn insert_at(show: &Show, row: &Row, g: &Graph, node: usize, link: usize) -> Result<Vec<Edit>, String> {
    let n = g.nodes.get(node).ok_or("no such node")?;
    let on = link_wire(g, link).ok_or("no such wire")?;
    if !row.cells.contains_key(&n.id) {
        return Err(format!("{} is not put on a wire: only a cell of the row is", n.title));
    }
    insert(show, row, &n.id, &on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::nodes::{row_graph, ROW};
    use crate::rows::{edit, edit_all};

    const SHOW: &str = r#"{ "id": "w", "version": 1, "toons": ["catpope"],
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "sky" }, { "id": "lights" }],
        "rows": [{ "id": "one", "fire": { "event": "ch1.enter" }, "cells": {
            "wall": { "mode": "play", "piece": "character:keys", "values": { "speed": 0.5 } },
            "floor": { "mode": "play", "piece": "character:bells", "values": { "glow": 1.0 } },
            "sky": { "mode": "play", "piece": "character:keys" },
            "lights": { "mode": "black" } } }] }"#;

    fn show() -> Show {
        parse(SHOW).unwrap()
    }

    fn wired(wires: &[CellWire]) -> Show {
        let mut s = show();
        s.rows[0].wires = wires.to_vec();
        s
    }

    /// a pin of the row's graph by node id and pin name
    fn at(g: &Graph, node: &str, out: bool, pin: &str) -> At {
        let i = g.node(node).unwrap();
        let pins = if out { &g.nodes[i].outs } else { &g.nodes[i].ins };
        (i, out, pins.iter().position(|p| p.label == pin).unwrap())
    }

    fn link(g: &Graph, w: &CellWire) -> usize {
        (0..g.links.len()).find(|&i| link_wire(g, i).as_ref() == Some(w)).unwrap()
    }

    #[test]
    fn a_drag_from_an_outlet_into_an_inlet_wires_them_and_a_refused_one_says_why() {
        let s = show();
        let g = row_graph(&s, 0, &[]);
        let held = take_at(&s, &s.rows[0], &g, at(&g, "wall", true, "ends")).unwrap();
        assert_eq!(held, Held { from: "wall".into(), out: "ends".into(), signal: Signal::Event, lifted: None });
        let lit = targets(&s, &s.rows[0], &g, &held);
        let fire = |n: &str| (g.node(n).unwrap(), 0);
        assert_eq!(lit, [fire("floor"), fire("sky"), fire("lights")], "only the inlets an event feeds, not its own cell's");
        let e = let_go_at(&s.rows[0], &g, &held, Some(at(&g, "floor", false, "fire"))).unwrap().unwrap();
        assert_eq!(e, Edit::Wire { row: "one".into(), wire: CellWire::new("wall", "ends", "floor", "fire"), replaces: None }, "the rows band's op");
        assert_eq!(edit(&s, &e).unwrap().show.rows[0].wires, [CellWire::new("wall", "ends", "floor", "fire")]);
        // a value takes no event: said while it is held over it, and refused in the same words when let go there
        let glow = at(&g, "floor", false, "glow");
        let why = lands_at(&s, &s.rows[0], &g, &held, glow).unwrap_err();
        assert_eq!(why, "wall ends → floor glow: event does not feed control");
        let e = let_go_at(&s.rows[0], &g, &held, Some(glow)).unwrap().unwrap();
        assert_eq!(edit(&s, &e).unwrap_err(), format!("not changed: row 'one': {why}"));
        // the row's own pins are no cell's
        assert!(take_at(&s, &s.rows[0], &g, (g.node(ROW).unwrap(), true, 0)).unwrap_err().contains("not a cell's port"));
        assert!(lands_at(&s, &s.rows[0], &g, &held, (g.node(ROW).unwrap(), false, 0)).unwrap_err().contains("not a cell's port"));
        assert_eq!(let_go_at(&s.rows[0], &g, &held, None), Ok(None), "a new wire let go on nothing: nothing");
    }

    #[test]
    fn a_wired_inlet_lifts_its_wire_to_plug_elsewhere_or_let_go_into_nothing() {
        let w = CellWire::new("wall", "ends", "floor", "fire");
        let s = wired(std::slice::from_ref(&w));
        let g = row_graph(&s, 0, &[]);
        let held = take_at(&s, &s.rows[0], &g, at(&g, "floor", false, "fire")).unwrap();
        assert_eq!(held.lifted.as_ref(), Some(&w));
        assert_eq!((held.from.as_str(), held.out.as_str(), held.signal), ("wall", "ends", Signal::Event));
        assert_eq!(let_go_at(&s.rows[0], &g, &held, Some(at(&g, "floor", false, "fire"))), Ok(None), "back where it was");
        let moved = let_go_at(&s.rows[0], &g, &held, Some(at(&g, "sky", false, "fire"))).unwrap().unwrap();
        assert_eq!(edit(&s, &moved).unwrap().show.rows[0].wires, [CellWire::new("wall", "ends", "sky", "fire")], "re-plugged, the old one gone");
        let gone = let_go_at(&s.rows[0], &g, &held, None).unwrap().unwrap();
        assert_eq!(gone, Edit::Unwire { row: "one".into(), wire: w });
        assert!(edit(&s, &gone).unwrap().show.rows[0].wires.is_empty());
        assert!(take_at(&s, &s.rows[0], &g, at(&g, "sky", false, "fire")).unwrap_err().contains("drag from an outlet"), "an unwired inlet lifts nothing");
    }

    #[test]
    fn a_cut_takes_the_hand_wires_it_crossed_in_one_step_and_leaves_the_rows_own() {
        let (a, b) = (CellWire::new("wall", "ends", "floor", "fire"), CellWire::new("floor", "ends", "sky", "fire"));
        let s = wired(&[a.clone(), b.clone()]);
        let g = row_graph(&s, 0, &[]);
        let fires = (0..g.links.len()).find(|&i| g.links[i].from.0 == g.node(ROW).unwrap()).unwrap();
        let edits = cut_at(&s.rows[0], &g, &[link(&g, &a), fires, link(&g, &b), link(&g, &a)]).unwrap();
        assert_eq!(edits, [Edit::Unwire { row: "one".into(), wire: a.clone() }, Edit::Unwire { row: "one".into(), wire: b }]);
        let done = edit_all(&s, &edits).unwrap();
        assert!(done.show.rows[0].wires.is_empty());
        assert_eq!(done.said, "row 1 one: unwired wall ends → floor fire; row 1 one: unwired floor ends → sky fire");
        assert!(cut_at(&s.rows[0], &g, &[fires]).unwrap_err().starts_with("nothing cut"), "the row fires its cells itself");
        assert_eq!(cut_at(&s.rows[0], &g, &[]), Ok(vec![]), "nothing crossed");
        let one = edit_all(&s, &cut_at(&s.rows[0], &g, &[link(&g, &a)]).unwrap()).unwrap();
        assert_eq!(one.show.rows[0].wires.len(), 1);
    }

    #[test]
    fn a_cell_dropped_on_a_wire_stands_between_its_ends_in_one_step() {
        let w = CellWire::new("wall", "ends", "floor", "fire");
        let s = wired(std::slice::from_ref(&w));
        let g = row_graph(&s, 0, &[]);
        let edits = insert_at(&s, &s.rows[0], &g, g.node("sky").unwrap(), link(&g, &w)).unwrap();
        assert!(matches!(&edits[..], [Edit::Unwire { .. }, Edit::Wire { .. }, Edit::Wire { .. }]), "one unwire, two wires: {edits:?}");
        let done = edit_all(&s, &edits).unwrap();
        assert_eq!(done.show.rows[0].wires, [CellWire::new("wall", "ends", "sky", "fire"), CellWire::new("sky", "ends", "floor", "fire")]);
        assert!(crate::validate(&done.show).is_empty());
        // an end of the wire, a cell without a fitting outlet, the row's own wire, a node that is no cell
        assert!(insert_at(&s, &s.rows[0], &g, g.node("wall").unwrap(), link(&g, &w)).unwrap_err().contains("is an end of"));
        assert_eq!(insert_at(&s, &s.rows[0], &g, g.node("lights").unwrap(), link(&g, &w)).unwrap_err(), "lights has no outlet that feeds event");
        let fires = (0..g.links.len()).find(|&i| g.links[i].from.0 == g.node(ROW).unwrap()).unwrap();
        assert!(insert_at(&s, &s.rows[0], &g, g.node("sky").unwrap(), fires).unwrap_err().contains("only on a wire drawn by hand"));
        assert!(insert_at(&s, &s.rows[0], &g, g.node(ROW).unwrap(), link(&g, &w)).unwrap_err().contains("only a cell of the row"));
    }

    #[test]
    fn several_edits_are_one_change_or_none() {
        let s = show();
        let wire = |to: &str| Edit::Wire { row: "one".into(), wire: CellWire::new("wall", "ends", to, "fire"), replaces: None };
        assert_eq!(edit_all(&s, &[wire("floor"), wire("sky")]).unwrap().show.rows[0].wires.len(), 2);
        assert!(edit_all(&s, &[wire("floor"), wire("floor")]).unwrap_err().contains("wired already"), "the second refused: nothing made");
    }
}
