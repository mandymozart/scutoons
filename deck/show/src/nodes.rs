//! A row as a node graph (the Show workspace's node view): what comes from far (the reader's story events, the show's
//! clock), the row that fires, the cells it changes on each outlet, what it sends to addresses. Each pin carries its
//! signal (deck-kit signal.rs), so the view types its inlets, outlets and wires by colour and shape. A kept cell plays on
//! from above: it stands folded, unwired. A far node of a cell's piece hangs from the cell whose piece holds it (far.rs).
//! The row's canvas (canvas.rs) folds nodes and groups; the conductor says what runs on each wire this frame.
use crate::canvas::{self, GROUP};
use crate::conductor::{Conductor, Logged, Playing};
use crate::document::{Cell, Moment, Show};
use crate::far::{Holder, Link as FarLink};
use crate::view;
use crate::wiring;
use deck_kit::base::graph::{Activity, Graph, Link, Node, Pin};
use deck_kit::base::signal::Signal;

pub const READER: &str = "reader";
pub const CLOCK: &str = "clock";
pub const ROW: &str = "row";
pub const SENDS: &str = "sends";
/// how long an event's pulse runs over its wires, ms: the first half from far into the row, the second out of it
pub const PULSE_MS: u64 = 900;

/// The graph of row `row` of `show`; `far` = the row's far nodes and their holders (far.rs `links`).
pub fn row_graph(show: &Show, row: usize, far: &[FarLink]) -> Graph {
    let mut g = Graph::default();
    let Some(r) = show.rows.get(row) else { return g };
    let events: Vec<&str> = {
        let mut e: Vec<&str> = Vec::new();
        for x in show.rows.iter().flat_map(|r| r.fire.event.iter()) {
            if !e.contains(&x) {
                e.push(x);
            }
        }
        e
    };
    if !events.is_empty() {
        let outs = events.iter().map(|e| Pin::new(*e, Signal::Event)).collect();
        g.nodes.push(Node { id: READER.into(), title: READER.into(), kind: "reader", detail: "from far".into(), outs, far: true, ..Node::default() });
    }
    g.nodes.push(Node { id: CLOCK.into(), title: CLOCK.into(), kind: "clock", detail: "show".into(), outs: vec![Pin::new("beat", Signal::Clock), Pin::new("bar", Signal::Clock)], far: true, ..Node::default() });
    let mut ins = Vec::new();
    if !r.fire.event.is_empty() {
        ins.push(Pin::new("on event", Signal::Event));
    }
    if r.fire.moment != Moment::Now {
        ins.push(Pin::new(format!("on {}", view::moment_text(r.fire.moment)), Signal::Clock));
    }
    let title = if r.fire.event.is_empty() { r.id.clone() } else { r.fire.event.words() };
    g.nodes.push(Node { id: ROW.into(), title, kind: "event", detail: view::how_text(r.fire.how).into(), ins, outs: vec![Pin::new("fire", Signal::Event)], ..Node::default() });
    let row_at = g.nodes.len() - 1;
    let wire = |g: &mut Graph, from: (usize, usize), to: (usize, usize), signal: Signal| g.links.push(Link::new(from, to, signal));
    // every event of the row into its one "on event"
    if let Some(reader) = g.node(READER) {
        for e in r.fire.event.iter() {
            if let Some(k) = events.iter().position(|x| *x == e) {
                wire(&mut g, (reader, k), (row_at, 0), Signal::Event);
            }
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
        let mut node = Node { id: o.id.clone(), title: o.name().to_string(), kind: kind_of(o.kind, &cell), detail: view::cell_text(&cell), collapsed: kept, ins, outs, ..Node::default() };
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
        g.nodes.push(Node { id: SENDS.into(), title: format!("this row sends · {}", r.addresses.len()), kind: "row", detail: format!("{} sends", r.addresses.len()), ins: vec![Pin::new("fire", Signal::Event)], outs, ..Node::default() });
        let at = g.nodes.len() - 1;
        wire(&mut g, (row_at, 0), (at, 0), Signal::Event);
    }
    // the canvas: folded by hand, framed in a group; what a cell plays, for its preview
    for n in &mut g.nodes {
        let id = n.id.clone();
        n.collapsed = canvas::is_folded(show, r, &id);
        n.group = r.nodes.groups.iter().position(|grp| grp.nodes.contains(&id));
        n.picture = match r.cells.get(&id) {
            Some(Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. }) => Some(p.clone()),
            _ => None,
        };
    }
    for i in 0..g.links.len() {
        let value = value_of(&g, &g.links[i], r);
        g.links[i].value = value;
    }
    g
}

/// a cell's node's kind: the place its piece is (`scene`, `character`, `item`), else its outlet's
fn kind_of(outlet: crate::document::OutletKind, cell: &Cell) -> &'static str {
    let piece = match cell {
        Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. } => p.split_once(':').map(|(place, _)| place),
        _ => None,
    };
    match (piece, outlet) {
        (Some("scene"), _) => "scene",
        (Some("character"), _) => "character",
        (Some("item"), _) => "item",
        (_, crate::document::OutletKind::Device) => "device",
        (_, crate::document::OutletKind::Stage) => "stage",
    }
}

/// what a wire carries, for people: a control's value as the cell sets it, else what its outlet gives
fn value_of(g: &Graph, l: &Link, r: &crate::document::Row) -> Option<String> {
    let (from, to) = (g.nodes.get(l.from.0)?, g.nodes.get(l.to.0)?);
    let out = from.outs.get(l.from.1).map(|p| p.label.clone()).unwrap_or_default();
    let inlet = to.ins.get(l.to.1).map(|p| p.label.clone()).unwrap_or_default();
    if l.signal == Signal::Control {
        if let Some(Cell::Play { values, .. } | Cell::Values { values, .. }) = r.cells.get(&to.id) {
            if let Some(v) = values.get(&inlet) {
                return Some(format!("{inlet} = {v}"));
            }
        }
    }
    // an event is its name: the inlet it reaches (`on event`, `fire`) says nothing more
    match (out.is_empty(), inlet.is_empty() || inlet == out || l.signal == Signal::Event) {
        (true, _) => Some(inlet).filter(|i| !i.is_empty()),
        (false, true) => Some(out),
        (false, false) => Some(format!("{out} → {inlet}")),
    }
}

/// The row's groups as the view frames them: each folded group's nodes in one pill (`group:<id>`), the open ones as a
/// list of their nodes; by the group's index in the row's canvas.
pub fn folded(g: Graph, show: &Show, row: usize) -> (Graph, Vec<(usize, Vec<usize>)>) {
    let Some(r) = show.rows.get(row) else { return (g, Vec::new()) };
    let mut g = g;
    for (k, grp) in r.nodes.groups.iter().enumerate().filter(|(_, x)| x.folded) {
        let members: Vec<usize> = (0..g.nodes.len()).filter(|&i| g.nodes[i].group == Some(k)).collect();
        if members.is_empty() {
            continue;
        }
        let pill = Node { id: format!("{GROUP}{}", grp.id), title: grp.label.clone(), kind: "group", detail: format!("{} nodes", members.len()), group: Some(k), ..Node::default() };
        g = g.fold(&members, pill);
    }
    let frames = (0..r.nodes.groups.len()).map(|k| (k, (0..g.nodes.len()).filter(|&i| g.nodes[i].group == Some(k)).collect::<Vec<_>>())).filter(|(_, m)| !m.is_empty()).collect();
    (g, frames)
}

/// What runs on the row's wires at `ms` (the conductor's clock): the row fired (or a cell of it launched) a moment ago –
/// its events pulse out from far into the row, then from the row on; a cell of this row on stage – the wires into and out
/// of it stream; the row is the current one – its clock streams. Anything else is idle.
pub fn activity(g: &mut Graph, show: &Show, row: usize, c: &Conductor, ms: u64) {
    let Some(r) = show.rows.get(row) else { return };
    let fired = c
        .log()
        .filter_map(|e| match &e.what {
            Logged::Fired { row, .. } | Logged::CellLaunched { row, .. } if *row == r.id => Some(e.ms),
            _ => None,
        })
        .max();
    let age = fired.map(|f| ms.saturating_sub(f) as f32 / PULSE_MS as f32).filter(|t| *t < 1.0);
    let playing = |id: &str| c.on_stage(id).is_some_and(|s| s.row == r.id && !s.queued(ms) && !matches!(s.playing, Playing::Black));
    let current = c.current() == Some(r.id.as_str());
    let far: Vec<bool> = g.nodes.iter().map(|n| n.far).collect();
    let on: Vec<bool> = g.nodes.iter().map(|n| playing(&n.id)).collect();
    for l in &mut g.links {
        l.activity = match l.signal {
            Signal::Event | Signal::Address | Signal::Vote => {
                let hop = if far.get(l.from.0) == Some(&true) { 0.0 } else { 1.0 };
                match age.map(|t| t * 2.0 - hop) {
                    Some(p) if (0.0..1.0).contains(&p) => Activity::Pulse(p),
                    _ => Activity::Idle,
                }
            }
            Signal::Clock if current => Activity::Stream,
            Signal::Clock => Activity::Idle,
            _ if on.get(l.from.0) == Some(&true) || on.get(l.to.0) == Some(&true) => Activity::Stream,
            _ => Activity::Idle,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::far::{links, tests::reach, tests::SHOW};

    #[test]
    fn a_row_fires_from_its_event_into_the_cells_it_changes() {
        let show = parse(SHOW).unwrap();
        let row = show.rows.iter().position(|r| !r.fire.event.is_empty()).unwrap();
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
    fn a_fired_row_pulses_from_far_into_the_row_then_on_and_its_cells_stream() {
        let show = parse(SHOW).unwrap();
        let (i, r) = show.rows.iter().enumerate().find(|(_, r)| !r.fire.event.is_empty()).unwrap();
        let mut c = Conductor::new(show.clone());
        let mut g = row_graph(&show, i, &[]);
        activity(&mut g, &show, i, &c, 1_000);
        assert!(g.links.iter().all(|l| l.activity == Activity::Idle), "nothing fired: all idle");
        c.fire(&r.id, crate::conductor::Now::at(1_000));
        let at = g.node(ROW).unwrap();
        let early = |g: &Graph| g.links.iter().find(|l| l.to.0 == at && l.signal == Signal::Event).map(|l| l.activity);
        let late = |g: &Graph| g.links.iter().find(|l| l.from.0 == at && l.signal == Signal::Event).map(|l| l.activity);
        activity(&mut g, &show, i, &c, 1_000 + PULSE_MS / 4);
        assert!(matches!(early(&g), Some(Activity::Pulse(p)) if (p - 0.5).abs() < 1e-3), "{:?}", early(&g));
        assert_eq!(late(&g), Some(Activity::Idle), "the row's own wires wait for the pulse");
        activity(&mut g, &show, i, &c, 1_000 + PULSE_MS * 3 / 4);
        assert!(matches!(late(&g), Some(Activity::Pulse(_))));
        activity(&mut g, &show, i, &c, 1_000 + PULSE_MS * 2);
        assert!(g.links.iter().all(|l| !matches!(l.activity, Activity::Pulse(_))), "the pulse is over");
        let streams = g.links.iter().filter(|l| l.activity == Activity::Stream).count();
        let on = show.outlets.iter().filter(|o| c.on_stage(&o.id).is_some_and(|s| s.row == r.id && !matches!(s.playing, Playing::Black))).count();
        assert_eq!(streams > 0, on > 0 && g.links.iter().any(|l| l.signal != Signal::Event && l.signal != Signal::Clock), "a cell on stage streams");
    }

    #[test]
    fn the_canvas_folds_nodes_and_groups_and_names_the_previews() {
        let show = parse(SHOW).unwrap();
        let row = show.rows[0].id.clone();
        let s = canvas::edit(&show, &row, &canvas::Edit::Fold { node: ROW.into() }).unwrap().show;
        let g = row_graph(&s, 0, &[]);
        assert!(g.nodes[g.node(ROW).unwrap()].collapsed, "folded by hand");
        let s = canvas::edit(&s, &row, &canvas::Edit::Group { nodes: vec![READER.into(), CLOCK.into()] }).unwrap().show;
        let (g, frames) = folded(row_graph(&s, 0, &[]), &s, 0);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].1.len(), 2, "an open group frames its two nodes");
        let id = s.rows[0].nodes.groups[0].id.clone();
        let s = canvas::edit(&s, &row, &canvas::Edit::Fold { node: format!("{GROUP}{id}") }).unwrap().show;
        let whole = row_graph(&s, 0, &[]);
        let (g2, frames) = folded(whole.clone(), &s, 0);
        assert_eq!(g2.nodes.len(), whole.nodes.len() - 1, "two nodes, one pill");
        let pill = g2.node(&format!("{GROUP}{id}")).unwrap();
        assert!(g2.nodes[pill].collapsed && frames[0].1 == [pill]);
        assert!(g.nodes.len() > g2.nodes.len());
        for (n, cell) in g2.nodes.iter().filter_map(|n| s.rows[0].cells.get(&n.id).map(|c| (n, c))) {
            if let Cell::Play { piece: Some(p), .. } = cell {
                assert_eq!(n.picture.as_deref(), Some(p.as_str()));
            }
        }
    }

    #[test]
    fn a_wire_says_what_it_carries() {
        let show = parse(SHOW).unwrap();
        let (i, r) = show.rows.iter().enumerate().find(|(_, r)| !r.fire.event.is_empty()).unwrap();
        let g = row_graph(&show, i, &[]);
        let into_row = g.links.iter().find(|l| l.to.0 == g.node(ROW).unwrap()).unwrap();
        assert_eq!(into_row.value.as_deref().map(|v| v.split(" → ").next().unwrap()), r.fire.event.first(), "the event's name");
        assert!(g.links.iter().all(|l| l.value.is_some()));
    }

    #[test]
    fn values_are_control_inlets_and_a_row_without_event_or_moment_has_no_inlet() {
        let show = parse(SHOW).unwrap();
        for (i, r) in show.rows.iter().enumerate() {
            let g = row_graph(&show, i, &[]);
            let at = g.node(ROW).unwrap();
            assert_eq!(g.nodes[at].ins.is_empty(), r.fire.event.is_empty() && r.fire.moment == Moment::Now);
            for (o, cell) in &r.cells {
                if let Cell::Play { values, .. } | Cell::Values { values, .. } = cell {
                    let n = &g.nodes[g.node(o).unwrap()];
                    assert_eq!(n.ins.iter().filter(|p| p.signal == Signal::Control).count(), values.len());
                }
            }
        }
    }
}
