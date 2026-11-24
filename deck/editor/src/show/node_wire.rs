//! Wiring in the node view, by the same edits as the rows band (deck-show wire_ops.rs decides, this file draws and
//! forwards): a drag from an outlet to an inlet wires them, a wired inlet dragged lifts its wire (onto another inlet: it
//! moves, into nothing: it goes), ctrl + a right drag cuts the wires it crosses, a node dragged onto a wire stands
//! between its ends in one step. While a wire is in the hand the inlets it may go into light; over one it may not, and
//! where a drop was refused, the core's sentence stands at the pointer.
use super::node_paint::{faded, Cam};
use super::node_pick::DRAG;
use super::nodes::{NodeHit, NodeView};
use super::{Hit, Host};
use deck_kit::base::canvas;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::Direction;
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::Edit;
use deck_show::wire_ops::{self, At, Held};

/// how near the pointer is to a wire to be on it, px
const NEAR: f32 = 5.0;

/// what the hand holds in the node view
#[derive(Debug, Clone, PartialEq)]
pub enum Hand {
    /// a wire from an outlet, new or lifted off an inlet
    Wire(Held),
    /// a cut: its path on screen
    Cut(Vec<(f32, f32)>),
    /// a node pressed at `from`; carried once the pointer went `DRAG` away
    Node { node: String, from: (f32, f32) },
}


/// a check run's stroke: from, to, the press that starts it (none: a cut)
type Stroke = ((f32, f32), (f32, f32), Option<Hit>);

/// a short path across the wire near its middle, `half` long to each side (between two of its samples)
fn across(w: &Wire, half: f32) -> ((f32, f32), (f32, f32)) {
    let (m, a, b) = (w.at(0.47), w.at(0.46), w.at(0.48));
    let len = (b.0 - a.0).hypot(b.1 - a.1).max(1e-3);
    let n = (-(b.1 - a.1) / len * half, (b.0 - a.0) / len * half);
    ((m.0 - n.0, m.1 - n.1), (m.0 + n.0, m.1 + n.1))
}

impl NodeView {
    /// the pin under `p`
    pub(super) fn pin_at(&self, p: (f32, f32)) -> Option<At> {
        self.pins.iter().rev().find(|(_, r)| r.has(p.0, p.1)).map(|(at, _)| *at)
    }

    /// where pin `at` is on screen
    pub(super) fn pin_place(&self, at: At) -> Option<(f32, f32)> {
        self.pins.iter().find(|(a, _)| *a == at).map(|(_, r)| (r.x + r.w * 0.5, r.y + r.h * 0.5))
    }

    /// the wire under `p` (its link), where no node covers it
    pub(super) fn wire_at(&self, p: (f32, f32)) -> Option<usize> {
        if self.drawn.iter().any(|(_, r)| self.on_screen(*r).has(p.0, p.1)) {
            return None;
        }
        let near = self.wires.iter().map(|(i, w)| (*i, canvas::to_line(&w.points(), p))).filter(|(_, d)| *d < NEAR);
        near.min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
    }

    /// the wires (their links) the path crosses
    pub(super) fn crossed(&self, path: &[(f32, f32)]) -> Vec<usize> {
        self.wires.iter().filter(|(_, w)| canvas::crosses(path, &w.points())).map(|(i, _)| *i).collect()
    }

    /// the node in `hand`, once it is carried: its place in the graph
    fn carried(&self, hand: &Hand, cursor: (f32, f32)) -> Option<usize> {
        match hand {
            Hand::Node { node, from } if (cursor.0 - from.0).hypot(cursor.1 - from.1) >= DRAG => self.graph.node(node),
            _ => None,
        }
    }
}

impl Host {
    /// a pin pressed: the core says what the hand takes; nothing: its sentence, and the press is the node's
    pub(super) fn pin_press(&mut self, at: At) {
        let show = self.shown().clone();
        let Some(row) = show.rows.get(self.row) else { return };
        match wire_ops::take_at(&show, row, &self.nodes.graph, at) {
            Ok(held) => self.nodes.hand = Some(Hand::Wire(held)),
            Err(why) => {
                self.press(Hit::Nodes(NodeHit::Node(at.0)));
                self.status = why;
            }
        }
    }

    /// ctrl + the right button pressed on the canvas: a cut starts
    pub fn nodes_cut(&mut self, x: f32, y: f32) -> bool {
        if !self.two_d || self.sheet.is_some() || !self.nodes.shown || !self.nodes.canvas.has(x, y) {
            return false;
        }
        self.nodes.hand = Some(Hand::Cut(vec![(x, y)]));
        true
    }

    /// the pointer moved with a cut in the hand: its path grows
    pub(super) fn hand_move(&mut self, at: (f32, f32)) {
        if let Some(Hand::Cut(path)) = self.nodes.hand.as_mut() {
            if path.last().is_none_or(|l| (l.0 - at.0).hypot(l.1 - at.1) >= 2.0) {
                path.push(at);
            }
        }
    }

    /// what letting the hand go here would change, as the core says: the edits (none: nothing), or why not
    fn hand_edits(&self, hand: &Hand) -> Result<Vec<Edit>, String> {
        let (n, show) = (&self.nodes, self.shown());
        let Some(row) = show.rows.get(self.row) else { return Ok(Vec::new()) };
        match hand {
            Hand::Wire(held) => wire_ops::let_go_at(row, &n.graph, held, n.pin_at(self.cursor)).map(|e| e.into_iter().collect()),
            Hand::Cut(path) => wire_ops::cut_at(row, &n.graph, &n.crossed(path)),
            Hand::Node { .. } => match (n.carried(hand, self.cursor), n.wire_at(self.cursor)) {
                (Some(node), Some(link)) => wire_ops::insert_at(show, row, &n.graph, node, link),
                _ => Ok(Vec::new()),
            },
        }
    }

    /// The button let go with something in the hand: the core's edits are made as one step; refused, its sentence
    /// stays where the pointer is.
    pub(super) fn hand_release(&mut self) {
        let Some(hand) = self.nodes.hand.take() else { return };
        let done = match self.hand_edits(&hand) {
            Ok(edits) if edits.is_empty() => return,
            Ok(_) if self.locked => Err(super::live::LOCKED.to_string()),
            Ok(edits) => Ok(edits),
            Err(why) => Err(why),
        };
        let refused = match done {
            Ok(edits) => !self.edit_rows_all(&edits),
            Err(why) => {
                self.refuse(why);
                true
            }
        };
        if refused {
            self.refused = Some(self.cursor);
        }
    }

    /// A check run's gesture (`--node-wire`), the pointer's own path through press, move and release:
    /// `wire:FROM.OUT>TO.IN`, `lift:TO.IN>TO.IN` (or `lift:TO.IN>` into nothing), `cut:FROM.OUT>TO.IN`,
    /// `insert:NODE@FROM.OUT>TO.IN`; `,hold` keeps the button down over the target, `,air` on the way to it.
    pub(super) fn script_node_wire(&mut self, spec: &str, release: bool) {
        let (spec, stay) = match spec.rsplit_once(',') {
            Some((s, "hold")) => (s, Some(1.0)),
            Some((s, "air")) => (s, Some(0.6)),
            _ => (spec, None),
        };
        if release {
            if stay.is_none() {
                self.release();
            }
            let wires: Vec<String> = self.shown().rows.get(self.row).map(|r| r.wires.iter().map(|w| w.words()).collect()).unwrap_or_default();
            tracing::info!("node-wire {spec}: wires [{}] · {}", wires.join(", "), self.status);
            return;
        }
        let Some((kind, rest)) = spec.split_once(':') else { return tracing::error!("--node-wire: KIND:… expected, not {spec}") };
        let g = self.nodes.graph.clone();
        let pin = |s: &str, out: bool| -> Option<At> {
            let (node, label) = s.split_once('.')?;
            let i = g.node(node)?;
            let pins = if out { &g.nodes[i].outs } else { &g.nodes[i].ins };
            Some((i, out, pins.iter().position(|p| p.label == label)?))
        };
        let link = |s: &str| -> Option<usize> {
            let (from, to) = s.split_once('>')?;
            let (a, b) = (pin(from, true)?, pin(to, false)?);
            g.links.iter().position(|l| l.from == (a.0, a.2) && l.to == (b.0, b.2))
        };
        let wire = |host: &Host, i: usize| host.nodes.wires.iter().find(|(k, _)| *k == i).map(|(_, w)| *w);
        let path: Option<Stroke> = match kind {
            "wire" | "lift" => rest.split_once('>').and_then(|(a, b)| {
                let from = pin(a, kind == "wire")?;
                let at = self.nodes.pin_place(from)?;
                let to = match b.is_empty() {
                    true => (at.0, self.nodes.canvas.y + self.nodes.canvas.h - 12.0),
                    false => self.nodes.pin_place(pin(b, false)?)?,
                };
                Some((at, to, Some(Hit::Nodes(NodeHit::Pin(from.0, from.1, from.2)))))
            }),
            "cut" => link(rest).and_then(|i| wire(self, i)).map(|w| {
                let (a, b) = across(&w, 18.0);
                (a, b, None)
            }),
            "insert" => rest.split_once('@').and_then(|(node, on)| {
                let i = g.node(node)?;
                let body = self.nodes.on_screen(self.nodes.drawn.get(i)?.1);
                let w = wire(self, link(on)?)?;
                let free = [0.5, 0.4, 0.6, 0.3, 0.7].into_iter().map(|t| w.at(t)).find(|p| self.nodes.wire_at(*p).is_some())?;
                Some(((body.x + body.w * 0.5, body.y + body.h * 0.5), free, Some(Hit::Nodes(NodeHit::Node(i)))))
            }),
            _ => None,
        };
        let Some((from, to, hit)) = path else { return tracing::error!("--node-wire: nothing to take for {spec} in this row's graph") };
        self.cursor = from;
        match hit {
            Some(h) => self.press(h),
            None => {
                self.nodes_cut(from.0, from.1);
            }
        }
        let k = stay.unwrap_or(1.0);
        self.pointer(from.0 + (to.0 - from.0) * 0.5 * k, from.1 + (to.1 - from.1) * 0.5 * k);
        self.pointer(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k);
    }
}

/// what the hand marks among the wires drawn: the lifted wire (hidden: it hangs from the hand), and the wires a let-go
/// here would take or stand on (lit)
pub fn marks(host: &Host) -> (Option<usize>, Vec<usize>) {
    let n = &host.nodes;
    let links = 0..n.graph.links.len();
    let Some(hand) = &n.hand else { return (None, Vec::new()) };
    let lifted = match hand {
        Hand::Wire(Held { lifted: Some(w), .. }) => links.clone().find(|&i| wire_ops::link_wire(&n.graph, i).as_ref() == Some(w)),
        _ => None,
    };
    let lit = match (hand, host.hand_edits(hand)) {
        (Hand::Cut(_) | Hand::Node { .. }, Ok(edits)) => {
            let gone = |i: &usize| edits.iter().any(|e| matches!(e, Edit::Unwire { wire, .. } if wire_ops::link_wire(&n.graph, *i).as_ref() == Some(wire)));
            links.filter(gone).collect()
        }
        _ => Vec::new(),
    };
    (lifted, lit)
}

/// The hand over the nodes: the wire from its outlet to the pointer with the inlets it may go into lit, the cut's
/// path, the carried node's outline; the core's sentence at the pointer where a let-go would be refused. No hand: the
/// pin under the pointer is ready.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &Host, cam: &Cam, canvas: Rect) {
    let (n, cursor, z) = (&host.nodes, host.cursor, cam.view.zoom);
    let r = t.size.rem(0.32) * z;
    let Some(hand) = &n.hand else {
        if let Some((at, p)) = n.pin_at(cursor).and_then(|at| Some((at, n.pin_place(at)?))) {
            let pin = n.graph.nodes.get(at.0).and_then(|x| if at.1 { x.outs.get(at.2) } else { x.ins.get(at.2) });
            if let Some(pin) = pin {
                d.disc(p.0, p.1, r * 2.2, faded(t.color.signals.of(pin.signal), 0.22));
            }
        }
        return;
    };
    let show = host.shown();
    let Some(row) = show.rows.get(host.row) else { return };
    let say = |d: &mut DrawList, why: &str| controls::chip(d, t, (cursor.0 + 12.0, cursor.1 + 10.0), canvas.x + canvas.w, why, true);
    match hand {
        Hand::Wire(held) => {
            let c = t.color.signals.of(held.signal);
            for (i, k) in wire_ops::targets(show, row, &n.graph, held) {
                if let Some(p) = n.pin_place((i, false, k)) {
                    d.disc(p.0, p.1, r * 2.2, faded(c, 0.25));
                    d.ring(p.0, p.1, r * 1.7, faded(c, 0.9));
                }
            }
            let from = n.graph.node(&held.from).and_then(|i| Some((i, true, n.graph.nodes[i].outs.iter().position(|p| p.label == held.out)?))).and_then(|at| n.pin_place(at));
            if let Some(from) = from {
                let step = cam.unit * z;
                let w = if cam.dir == Direction::Lr { Wire::hang(from, cursor, step) } else { Wire::drop(from, cursor, step) };
                d.wire(&w, 1.6 * z * held.signal.weight(), c, &[]);
                d.disc(cursor.0, cursor.1, r * 0.8, c);
            }
            if let Some(Err(why)) = n.pin_at(cursor).filter(|at| !at.1).map(|at| wire_ops::lands_at(show, row, &n.graph, held, at)) {
                say(d, &why);
            }
        }
        Hand::Cut(path) => {
            for p in path.windows(2) {
                d.segment(p[0], p[1], 1.5, t.color.danger);
            }
            if let Err(why) = host.hand_edits(hand) {
                say(d, &why);
            }
        }
        Hand::Node { from, .. } => {
            let Some(body) = n.carried(hand, cursor).and_then(|i| n.drawn.get(i)).map(|(_, b)| n.on_screen(*b)) else { return };
            let (x, y) = (body.x + cursor.0 - from.0, body.y + cursor.1 - from.1);
            let radius = t.size.radius * 1.5 * z;
            d.round_rect(x, y, body.w, body.h, radius, faded(t.color.text, 0.08));
            d.outline(x, y, body.w, body.h, (radius, 1.0), faded(t.color.text, 0.8));
            if let Err(why) = host.hand_edits(hand) {
                say(d, &why);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::rows::tests::{host_with, H, W};
    use super::*;
    use deck_kit::base::tokens::Theme;
    use deck_show::wiring::CellWire;

    const SHOW: &str = r#"{ "id": "wired", "version": 1, "toons": ["catpope"],
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "sky" }],
        "rows": [{ "id": "one", "cells": {
            "wall": { "mode": "play", "piece": "scene:keys", "values": { "speed": 0.5 } },
            "floor": { "mode": "play", "piece": "scene:bells", "values": { "glow": 1.0 } },
            "sky": { "mode": "play", "piece": "scene:keys" } } }] }"#;

    fn frame(host: &mut Host) -> DrawList {
        let (mut d, t) = (DrawList::new(0.6), Tokens::new(Theme::Night, 16.0));
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (-10.0, -10.0));
        d
    }

    fn host(wires: &[CellWire]) -> Host {
        let mut host = host_with(SHOW);
        let mut s = host.shown().clone();
        s.rows[0].wires = wires.to_vec();
        host.changed(s);
        frame(&mut host);
        host
    }

    fn pin(host: &Host, node: &str, out: bool, label: &str) -> At {
        let g = &host.nodes.graph;
        let i = g.node(node).unwrap();
        let pins = if out { &g.nodes[i].outs } else { &g.nodes[i].ins };
        (i, out, pins.iter().position(|p| p.label == label).unwrap())
    }

    /// the pointer pressed on a pin as the window would: by what lies under it
    fn press_pin(host: &mut Host, node: &str, out: bool, label: &str) {
        let at = pin(host, node, out, label);
        host.cursor = host.nodes.pin_place(at).expect("the pin on screen");
        let hit = host.hit(host.cursor.0, host.cursor.1).unwrap();
        assert_eq!(hit, Hit::Nodes(NodeHit::Pin(at.0, at.1, at.2)), "a pin lies over its node");
        host.press(hit);
    }

    fn wires(host: &Host) -> Vec<CellWire> {
        host.shown().rows[0].wires.clone()
    }

    fn link(host: &Host, w: &CellWire) -> Wire {
        let g = &host.nodes.graph;
        let i = (0..g.links.len()).find(|&i| wire_ops::link_wire(g, i).as_ref() == Some(w)).unwrap();
        host.nodes.wires.iter().find(|(k, _)| *k == i).unwrap().1
    }

    #[test]
    fn a_drag_from_an_outlet_into_an_inlet_wires_them_and_a_refused_drop_says_why_at_the_pointer() {
        let mut host = host(&[]);
        press_pin(&mut host, "wall", true, "ends");
        assert!(matches!(&host.nodes.hand, Some(Hand::Wire(h)) if h.lifted.is_none()));
        let glow = host.nodes.pin_place(pin(&host, "floor", false, "glow")).unwrap();
        host.pointer(glow.0, glow.1);
        let d = frame(&mut host);
        assert!(d.texts().contains(&"wall ends → floor glow: event does not feed control"), "held over an inlet it does not feed: the core's sentence");
        host.release();
        assert!(wires(&host).is_empty());
        assert_eq!(host.refused, Some(glow), "refused where it was let go");
        assert!(host.bad && host.status.ends_with("event does not feed control"), "{}", host.status);
        frame(&mut host);
        press_pin(&mut host, "wall", true, "ends");
        let fire = host.nodes.pin_place(pin(&host, "floor", false, "fire")).unwrap();
        host.pointer(fire.0, fire.1);
        assert!(!frame(&mut host).texts().iter().any(|x| x.contains("does not feed")));
        host.release();
        assert_eq!(wires(&host), [CellWire::new("wall", "ends", "floor", "fire")], "{}", host.status);
        assert!(host.nodes.hand.is_none());
    }

    #[test]
    fn a_wired_inlet_lifts_its_wire_onto_another_inlet_or_into_nothing() {
        let mut host = host(&[CellWire::new("wall", "ends", "floor", "fire")]);
        press_pin(&mut host, "floor", false, "fire");
        assert!(matches!(&host.nodes.hand, Some(Hand::Wire(h)) if h.lifted.is_some()));
        assert!(marks(&host).0.is_some(), "the lifted wire hangs from the hand, not from its inlet");
        let sky = host.nodes.pin_place(pin(&host, "sky", false, "fire")).unwrap();
        host.pointer(sky.0, sky.1);
        host.release();
        assert_eq!(wires(&host), [CellWire::new("wall", "ends", "sky", "fire")], "{}", host.status);
        frame(&mut host);
        press_pin(&mut host, "sky", false, "fire");
        let c = host.nodes.canvas;
        host.pointer(c.x + 3.0, c.y + c.h - 3.0);
        host.release();
        assert!(wires(&host).is_empty(), "{}", host.status);
    }

    #[test]
    fn ctrl_and_a_right_drag_cuts_the_wires_it_crosses_in_one_step() {
        let (a, b) = (CellWire::new("wall", "ends", "floor", "fire"), CellWire::new("wall", "ends", "sky", "fire"));
        let mut host = host(&[a.clone(), b.clone()]);
        let (over_a, over_b) = (across(&link(&host, &a), 6.0), across(&link(&host, &b), 6.0));
        assert!(host.nodes_cut(over_a.0 .0, over_a.0 .1));
        for p in [over_a.1, over_b.0, over_b.1] {
            host.pointer(p.0, p.1);
        }
        assert_eq!(marks(&host).1.len(), 2, "both lit while the cut is held");
        host.release();
        assert!(wires(&host).is_empty(), "{}", host.status);
        assert_eq!(host.status.matches("unwired").count(), 2, "one change for both: {}", host.status);
        // only the row's own wire crossed: refused in the core's words
        frame(&mut host);
        let g = host.nodes.graph.clone();
        let own = host.nodes.wires.iter().find(|(i, _)| g.nodes[g.links[*i].from.0].id == deck_show::nodes::ROW).unwrap().1;
        let (from, to) = across(&own, 4.0);
        assert!(host.nodes_cut(from.0, from.1));
        host.pointer(to.0, to.1);
        host.release();
        assert!(host.bad && host.status.starts_with("nothing cut"), "{}", host.status);
        assert!(!host.nodes_cut(-5.0, -5.0), "beside the canvas no cut starts");
    }

    #[test]
    fn a_node_dragged_onto_a_wire_stands_between_its_ends_in_one_step() {
        let w = CellWire::new("wall", "ends", "floor", "fire");
        let mut host = host(std::slice::from_ref(&w));
        let sky = host.nodes.graph.node("sky").unwrap();
        let body = host.nodes.on_screen(host.nodes.drawn[sky].1);
        let on = [0.5, 0.4, 0.6, 0.3, 0.7].into_iter().map(|t| link(&host, &w).at(t)).find(|p| host.nodes.wire_at(*p).is_some()).expect("the wire clear of the nodes");
        host.cursor = (body.x + body.w * 0.5, body.y + 6.0);
        host.press(Hit::Nodes(NodeHit::Node(sky)));
        host.pointer(on.0, on.1);
        assert_eq!(marks(&host).1.len(), 1, "the wire it would stand on is lit");
        host.release();
        assert_eq!(wires(&host), [CellWire::new("wall", "ends", "sky", "fire"), CellWire::new("sky", "ends", "floor", "fire")], "{}", host.status);
        assert_eq!(host.status, "row 1 one: unwired wall ends → floor fire; row 1 one: wired wall ends → sky fire; row 1 one: wired sky ends → floor fire", "one change");
        // a click without a drag only selects; a node dropped beside every wire changes nothing
        frame(&mut host);
        host.press(Hit::Nodes(NodeHit::Node(sky)));
        host.release();
        let c = host.nodes.canvas;
        host.press(Hit::Nodes(NodeHit::Node(sky)));
        host.pointer(c.x + 3.0, c.y + c.h - 3.0);
        host.release();
        assert_eq!(wires(&host).len(), 2);
        assert!(!host.bad, "{}", host.status);
    }

    #[test]
    fn a_locked_show_takes_no_wire_cut_or_insert() {
        let w = CellWire::new("wall", "ends", "floor", "fire");
        let mut host = host(std::slice::from_ref(&w));
        host.locked = true;
        let (from, to) = across(&link(&host, &w), 6.0);
        assert!(host.nodes_cut(from.0, from.1));
        host.pointer(to.0, to.1);
        host.release();
        assert_eq!(wires(&host), [w]);
        assert_eq!(host.status, super::super::live::LOCKED);
    }
}
