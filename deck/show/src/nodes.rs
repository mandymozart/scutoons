//! A row as a node graph (the Show workspace's node view): what comes from far (the reader's story events, the show's
//! clock), the row that fires, the cells it changes on each outlet, what it sends to addresses. Each pin carries its
//! signal (deck-kit signal.rs), so the view types its inlets, outlets and wires by colour and shape. A kept cell plays on
//! from above: it stands folded, unwired. A far node of a cell's piece hangs from the cell whose piece holds it (far.rs).
use crate::document::{Cell, Moment, Show};
use crate::wiring;
use crate::far::{Holder, Link as FarLink};
use crate::view;
use deck_kit::base::graph::{Graph, Link, Node, Pin};
use deck_kit::base::signal::Signal;

pub const READER: &str = "reader";
pub const CLOCK: &str = "clock";
pub const ROW: &str = "row";
pub const SENDS: &str = "sends";

/// The graph of row `row` of `show`; `far` = the row's far nodes and their holders (far.rs `links`).
pub fn row_graph(show: &Show, row: usize, far: &[FarLink]) -> Graph {
    let mut g = Graph::default();
    let Some(r) = show.rows.get(row) else { return g };
    let events: Vec<&str> = {
        let mut e: Vec<&str> = Vec::new();
        for x in show.rows.iter().filter_map(|r| r.fire.event.as_deref()) {
            if !e.contains(&x) {
                e.push(x);
            }
        }
        e
    };
    if !events.is_empty() {
        let outs = events.iter().map(|e| Pin::new(*e, Signal::Event)).collect();
        g.nodes.push(Node { id: READER.into(), title: READER.into(), detail: "from far".into(), outs, far: true, ..Node::default() });
    }
    g.nodes.push(Node { id: CLOCK.into(), title: CLOCK.into(), detail: "show".into(), outs: vec![Pin::new("beat", Signal::Clock), Pin::new("bar", Signal::Clock)], far: true, ..Node::default() });
    let mut ins = Vec::new();
    if r.fire.event.is_some() {
        ins.push(Pin::new("on event", Signal::Event));
    }
    if r.fire.moment != Moment::Now {
        ins.push(Pin::new(format!("on {}", view::moment_text(r.fire.moment)), Signal::Clock));
    }
    g.nodes.push(Node { id: ROW.into(), title: r.id.clone(), detail: view::how_text(r.fire.how).into(), ins, outs: vec![Pin::new("fire", Signal::Event)], ..Node::default() });
    let row_at = g.nodes.len() - 1;
    let wire = |g: &mut Graph, from: (usize, usize), to: (usize, usize), signal: Signal| g.links.push(Link { from, to, signal, live: true });
    if let (Some(e), Some(reader)) = (r.fire.event.as_deref(), g.node(READER)) {
        if let Some(k) = events.iter().position(|x| *x == e) {
            wire(&mut g, (reader, k), (row_at, 0), Signal::Event);
        }
    }
    if r.fire.moment != Moment::Now {
        let clock = g.node(CLOCK).unwrap_or(0);
        let on = g.nodes[row_at].ins.len() - 1;
        wire(&mut g, (clock, 1), (row_at, on), Signal::Clock);
    }

    for o in &show.outlets {
        let cell = r.cells.get(&o.id).cloned().unwrap_or(Cell::Keep);
        let kept = matches!(cell, Cell::Keep);
        let (ins, outs) = wiring::ports(show, &o.id, &cell);
        let mut node = Node { id: o.id.clone(), title: o.name().to_string(), detail: view::cell_text(&cell), collapsed: kept, ins, outs, ..Node::default() };
        // the far nodes' inlets after the cell's own, before a mirror's
        let mirrors = node.ins.iter().position(|p| p.signal == Signal::Scene).unwrap_or(node.ins.len());
        for l in far.iter().filter(|l| l.from == o.id) {
            let from = match &l.holder {
                Holder::Here => "this piece".to_string(),
                Holder::Outlet(h) => h.clone(),
                Holder::Away => "not in this row".to_string(),
            };
            let at = mirrors + node.ins.iter().filter(|p| p.signal == Signal::Far).count();
            node.ins.insert(at, Pin::new(format!("{} ← {from}", l.node), Signal::Far));
        }
        g.nodes.push(node);
        if !kept {
            let at = g.nodes.len() - 1;
            wire(&mut g, (row_at, 0), (at, 0), Signal::Event);
        }
    }
    // the cells' own wires: a far node from the cell that holds it, a mirror from the cell it shows
    for o in &show.outlets {
        let Some(to) = g.node(&o.id) else { continue };
        let far_pins: Vec<usize> = g.nodes[to].ins.iter().enumerate().filter(|(_, p)| p.signal == Signal::Far).map(|(i, _)| i).collect();
        for (l, &pin) in far.iter().filter(|l| l.from == o.id).zip(&far_pins) {
            let Holder::Outlet(h) = &l.holder else { continue };
            let Some(from) = g.node(h) else { continue };
            let out = g.nodes[from].outs.len();
            g.nodes[from].outs.push(Pin::new(l.node.clone(), Signal::Far));
            wire(&mut g, (from, out), (to, pin), Signal::Far);
        }
        if let Some(Cell::Mirror { of }) = r.cells.get(&o.id) {
            let (Some(from), Some(pin)) = (g.node(of), g.nodes[to].ins.iter().position(|p| p.signal == Signal::Scene)) else { continue };
            if !g.nodes[from].outs.is_empty() {
                wire(&mut g, (from, 0), (to, pin), Signal::Scene);
            }
        }
    }
    // the wires drawn by hand between the cells
    for p in wiring::placed(show, r) {
        if let (Some(from), Some(to)) = (g.node(&p.wire.from), g.node(&p.wire.to)) {
            wire(&mut g, (from, p.out_at), (to, p.in_at), p.signal);
        }
    }
    if !r.addresses.is_empty() {
        let outs = r.addresses.iter().map(|a| Pin::new(view::address_text(a), Signal::Address)).collect();
        g.nodes.push(Node { id: SENDS.into(), title: "this row sends".into(), detail: format!("{} sends", r.addresses.len()), ins: vec![Pin::new("fire", Signal::Event)], outs, ..Node::default() });
        let at = g.nodes.len() - 1;
        wire(&mut g, (row_at, 0), (at, 0), Signal::Event);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::far::{links, tests::reach, tests::SHOW};

    #[test]
    fn a_row_fires_from_its_event_into_the_cells_it_changes() {
        let show = parse(SHOW).unwrap();
        let row = show.rows.iter().position(|r| r.fire.event.is_some()).unwrap();
        let g = row_graph(&show, row, &[]);
        let (reader, at) = (g.node(READER).unwrap(), g.node(ROW).unwrap());
        assert!(g.links.iter().any(|l| l.from.0 == reader && l.to.0 == at && l.signal == Signal::Event), "the event fires the row");
        for o in &show.outlets {
            let n = g.node(&o.id).unwrap();
            let fired = g.links.iter().any(|l| l.from.0 == at && l.to.0 == n);
            assert_eq!(fired, show.rows[row].cells.contains_key(&o.id), "{}", o.id);
            assert_eq!(g.nodes[n].collapsed, !show.rows[row].cells.contains_key(&o.id), "a kept cell stands folded");
        }
        for l in &g.links {
            assert_eq!(g.nodes[l.from.0].outs[l.from.1].signal, l.signal, "an outlet of the wire's type");
            assert_eq!(g.nodes[l.to.0].ins[l.to.1].signal, l.signal, "into an inlet of its type");
        }
    }

    #[test]
    fn a_far_node_hangs_from_the_cell_whose_piece_holds_it() {
        let show = parse(SHOW).unwrap();
        let (i, row) = show.rows.iter().enumerate().find(|(_, r)| links(&show, &r.id, reach).iter().any(|l| matches!(l.holder, Holder::Outlet(_)))).unwrap();
        let far = links(&show, &row.id, reach);
        let g = row_graph(&show, i, &far);
        let l = far.iter().find(|l| matches!(l.holder, Holder::Outlet(_))).unwrap();
        let Holder::Outlet(h) = &l.holder else { unreachable!() };
        let wire = g.links.iter().find(|k| k.signal == Signal::Far).expect("a far wire");
        assert_eq!((g.nodes[wire.from.0].id.as_str(), g.nodes[wire.to.0].id.as_str()), (h.as_str(), l.from.as_str()));
    }

    #[test]
    fn values_are_control_inlets_and_a_row_without_event_or_moment_has_no_inlet() {
        let show = parse(SHOW).unwrap();
        for (i, r) in show.rows.iter().enumerate() {
            let g = row_graph(&show, i, &[]);
            let at = g.node(ROW).unwrap();
            assert_eq!(g.nodes[at].ins.is_empty(), r.fire.event.is_none() && r.fire.moment == Moment::Now);
            for (o, cell) in &r.cells {
                if let Cell::Play { values, .. } | Cell::Values { values, .. } = cell {
                    let n = &g.nodes[g.node(o).unwrap()];
                    assert_eq!(n.ins.iter().filter(|p| p.signal == Signal::Control).count(), values.len());
                }
            }
        }
    }
}
