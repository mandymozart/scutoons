//! A node view's graph and its layout: nodes with typed inlets and outlets (signal.rs), the links between them, laid
//! out in layers left â†’ right or top â†’ bottom (the workspace's setting), every node on the 1rem grid. A crate fills the
//! graph with its own words (deck-show: a row's nodes); a view lays it out here and draws it.
use crate::base::draw::Rect;
use crate::base::signal::Signal;
use crate::base::wire::Wire;
use serde::{Deserialize, Serialize};

/// which way the signal runs through a node view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// inlets on the left, outlets on the right
    #[default]
    Lr,
    /// inlets on top, outlets below
    Tb,
}

impl Direction {
    /// the flow as a unit vector (a triangle pin points along it)
    pub fn flow(self) -> (f32, f32) {
        match self {
            Direction::Lr => (1.0, 0.0),
            Direction::Tb => (0.0, 1.0),
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Direction::Lr => "LR",
            Direction::Tb => "TB",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub label: String,
    pub signal: Signal,
    /// what it holds now, for people (a control's value): drawn beside the label
    pub value: Option<String>,
}

impl Pin {
    pub fn new(label: impl Into<String>, signal: Signal) -> Pin {
        Pin { label: label.into(), signal, value: None }
    }

    pub fn valued(self, value: impl Into<String>) -> Pin {
        Pin { value: Some(value.into()), ..self }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    pub id: String,
    pub title: String,
    /// what kind of node, one word (`clock`, `event`, `character`): the head's small word and its icon
    pub kind: &'static str,
    /// where it comes from, what it does, for people (`from far`, `plays scene:nave`)
    pub detail: String,
    pub ins: Vec<Pin>,
    pub outs: Vec<Pin>,
    /// from outside this subtree: drawn quieter
    pub far: bool,
    /// folded to a pill with one pin per signal on each side
    pub collapsed: bool,
    /// what it plays, as a preview's key (`scene:nave`): the view shows its picture when it has one
    pub picture: Option<String>,
    /// the group it is framed in (the crate's index): a layer keeps a group's nodes together
    pub group: Option<usize>,
}

/// the id prefix of a folded group's pill (`group:<id>`): a node that stands for the group's members
pub const PILL: &str = "group:";

impl Node {
    /// the signal a node is known by – its colour and its tone: its first outlet's, else its first inlet's
    pub fn signal(&self) -> Option<Signal> {
        self.outs.first().or(self.ins.first()).map(|p| p.signal)
    }

    /// the signals of `pins`, each once, in the order they first come
    pub fn merged(pins: &[Pin]) -> Vec<Signal> {
        let mut out: Vec<Signal> = Vec::new();
        for p in pins {
            if !out.contains(&p.signal) {
                out.push(p.signal);
            }
        }
        out
    }
}

/// what runs on a wire this frame (the crate's core says it; the view draws it)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Activity {
    /// nothing: drawn faint
    #[default]
    Idle,
    /// a stream flows (a piece plays, the clock runs): it glows
    Stream,
    /// an event went through: lit, a pulse on its way to the inlet, 0 -> 1 as it runs
    Pulse(f32),
}

/// a wire: from an outlet (node, its outs' index) to an inlet (node, its ins' index)
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub signal: Signal,
    pub activity: Activity,
    /// what it carries, for people (the value on hovering it): `0.8`, `bar`, `yellowmouse-enters`
    pub value: Option<String>,
}

impl Link {
    pub fn new(from: (usize, usize), to: (usize, usize), signal: Signal) -> Link {
        Link { from, to, signal, activity: Activity::Idle, value: None }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

impl Graph {
    pub fn node(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Each node's layer: the longest run of links into it (a node nothing reaches is in layer 0). A link back to a node
    /// already passed (a loop) does not push it further.
    pub fn layers(&self) -> Vec<usize> {
        let n = self.nodes.len();
        let mut layer = vec![0usize; n];
        for _ in 0..n {
            let mut moved = false;
            for l in &self.links {
                let (a, b) = (l.from.0, l.to.0);
                if a < n && b < n && a != b && layer[b] < layer[a] + 1 && layer[a] + 1 < n {
                    layer[b] = layer[a] + 1;
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        layer
    }

    /// The graph with nodes `members` folded into `pill`: it stands collapsed after the others, each wire that crossed
    /// the fold now on a pin of its own (the pill merges them per signal); the wires between members go.
    pub fn fold(&self, members: &[usize], mut pill: Node) -> Graph {
        let inside = |i: usize| members.contains(&i);
        let mut out = Graph::default();
        let mut map = vec![usize::MAX; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate().filter(|(i, _)| !inside(*i)) {
            map[i] = out.nodes.len();
            out.nodes.push(n.clone());
        }
        let at = out.nodes.len();
        (pill.collapsed, pill.ins, pill.outs) = (true, Vec::new(), Vec::new());
        for l in self.links.iter().filter(|l| l.from.0 < map.len() && l.to.0 < map.len()) {
            let mut k = l.clone();
            match (inside(l.from.0), inside(l.to.0)) {
                (true, true) => continue,
                (false, false) => (k.from.0, k.to.0) = (map[l.from.0], map[l.to.0]),
                (true, false) => {
                    pill.outs.push(self.nodes[l.from.0].outs.get(l.from.1).cloned().unwrap_or_else(|| Pin::new("", l.signal)));
                    (k.from, k.to.0) = ((at, pill.outs.len() - 1), map[l.to.0]);
                }
                (false, true) => {
                    pill.ins.push(self.nodes[l.to.0].ins.get(l.to.1).cloned().unwrap_or_else(|| Pin::new("", l.signal)));
                    (k.from.0, k.to) = (map[l.from.0], (at, pill.ins.len() - 1));
                }
            }
            out.links.push(k);
        }
        out.nodes.push(pill);
        out
    }
}

/// the sizes a layout is made of, in canvas pixels (the zoom scales them)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// the grid every node snaps to (1rem)
    pub unit: f32,
    /// a node's least width
    pub width: f32,
    /// its head (the title) and each line of pins
    pub head: f32,
    pub line: f32,
    /// the room between two layers (where the wires hang) and between two nodes of a layer
    pub gap: f32,
    pub spacing: f32,
    /// the least room per pin along a top â†’ bottom node's edge
    pub pin_gap: f32,
    /// the room under an open node for its preview (0: no previews)
    pub preview: f32,
}

impl Metrics {
    /// the sizes for a UI whose rem is `unit`
    pub fn of(unit: f32) -> Metrics {
        Metrics { unit, width: 14.0 * unit, head: 2.0 * unit, line: 1.5 * unit, gap: 5.0 * unit, spacing: 2.0 * unit, pin_gap: 4.0 * unit, preview: 0.0 }
    }
}

/// where a node landed on the canvas and where each of its pins sits (the merged pin of its signal, when collapsed)
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub rect: Rect,
    pub ins: Vec<(f32, f32)>,
    pub outs: Vec<(f32, f32)>,
}

/// a node's size before it is placed
fn size(n: &Node, dir: Direction, m: &Metrics) -> (f32, f32) {
    let snap = |v: f32| (v / m.unit).ceil() * m.unit;
    let preview = if n.picture.is_some() { m.preview } else { 0.0 };
    let (ins, outs) = match n.collapsed {
        true => (Node::merged(&n.ins).len(), Node::merged(&n.outs).len()),
        false => (n.ins.len(), n.outs.len()),
    };
    match (dir, n.collapsed) {
        (Direction::Lr, true) => (snap(m.width), snap(m.head.max(ins.max(outs) as f32 * m.line * 0.6))),
        (Direction::Lr, false) => (snap(m.width), snap(m.head + ins.max(outs) as f32 * m.line + m.line * 0.4 + preview)),
        (Direction::Tb, true) => (snap(m.width.max(ins.max(outs) as f32 * m.pin_gap * 0.6)), snap(m.head)),
        (Direction::Tb, false) => (snap(m.width.max(ins.max(outs) as f32 * m.pin_gap)), snap(m.head + m.line + preview)),
    }
}

/// where `count` pins sit along an edge from `start`, `len` long: evenly, each in the middle of its share
fn along(start: f32, len: f32, count: usize) -> impl Iterator<Item = f32> {
    (0..count).map(move |i| start + len * (i as f32 + 0.5) / count as f32)
}

/// a node's inlets and outlets, where each sits
type Spots = (Vec<(f32, f32)>, Vec<(f32, f32)>);

fn pins(n: &Node, r: Rect, dir: Direction, m: &Metrics) -> Spots {
    let side = |pins: &[Pin], at_end: bool| -> Vec<(f32, f32)> {
        let edge = match (dir, at_end) {
            (Direction::Lr, false) => r.x,
            (Direction::Lr, true) => r.x + r.w,
            (Direction::Tb, false) => r.y,
            (Direction::Tb, true) => r.y + r.h,
        };
        let at = |c: f32| match dir {
            Direction::Lr => (edge, c),
            Direction::Tb => (c, edge),
        };
        if n.collapsed {
            let merged = Node::merged(pins);
            let (start, len) = match dir {
                Direction::Lr => (r.y, r.h),
                Direction::Tb => (r.x, r.w),
            };
            let spots: Vec<f32> = along(start, len, merged.len()).collect();
            return pins.iter().map(|p| at(spots[merged.iter().position(|s| *s == p.signal).unwrap_or(0)])).collect();
        }
        match dir {
            Direction::Lr => (0..pins.len()).map(|i| at(r.y + m.head + m.line * (i as f32 + 0.5) + m.line * 0.2)).collect(),
            Direction::Tb => along(r.x, r.w, pins.len()).map(at).collect(),
        }
    };
    (side(&n.ins, false), side(&n.outs, true))
}

/// The graph laid out `dir`: layer after layer along the flow, each layer's nodes side by side across it, ordered by
/// where what feeds them sits (fewer crossings), each layer centred on the widest; every corner on the grid.
pub fn layout(g: &Graph, dir: Direction, m: &Metrics) -> Vec<Placed> {
    let layer = g.layers();
    let count = layer.iter().copied().max().map_or(0, |l| l + 1);
    let sizes: Vec<(f32, f32)> = g.nodes.iter().map(|n| size(n, dir, m)).collect();
    // the order within a layer: the first by the graph's order, a later one by the mean place of what feeds it
    let mut rank = vec![0.0f32; g.nodes.len()];
    let mut order: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (l, ordered) in order.iter_mut().enumerate() {
        let mut here: Vec<(f32, usize)> = (0..g.nodes.len())
            .filter(|&i| layer[i] == l)
            .map(|i| {
                let feeds: Vec<f32> = g.links.iter().filter(|k| k.to.0 == i && k.from.0 < g.nodes.len() && layer[k.from.0] < l).map(|k| rank[k.from.0]).collect();
                let key = if feeds.is_empty() { i as f32 } else { feeds.iter().sum::<f32>() / feeds.len() as f32 };
                (key, i)
            })
            .collect();
        here.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        // a group's nodes follow its first one, so its frame holds them alone
        let mut kept: Vec<(f32, usize)> = Vec::with_capacity(here.len());
        for (k, i) in here {
            match g.nodes[i].group.and_then(|grp| kept.iter().rposition(|(_, j)| g.nodes[*j].group == Some(grp))) {
                Some(last) => kept.insert(last + 1, (k, i)),
                None => kept.push((k, i)),
            }
        }
        let here = kept;
        for (k, (_, i)) in here.iter().enumerate() {
            rank[*i] = k as f32;
        }
        *ordered = here.into_iter().map(|(_, i)| i).collect();
    }
    // along the flow: each layer as deep as its deepest node; across it: the layer's nodes and their spacing
    let deep = |s: (f32, f32)| if dir == Direction::Lr { s.0 } else { s.1 };
    let across = |s: (f32, f32)| if dir == Direction::Lr { s.1 } else { s.0 };
    let depth: Vec<f32> = order.iter().map(|o| o.iter().map(|&i| deep(sizes[i])).fold(0.0, f32::max)).collect();
    let span: Vec<f32> = order.iter().map(|o| o.iter().map(|&i| across(sizes[i])).sum::<f32>() + m.spacing * o.len().saturating_sub(1) as f32).collect();
    let widest = span.iter().copied().fold(0.0, f32::max);
    let snap = |v: f32| (v / m.unit).round() * m.unit;
    let mut at = vec![Rect::default(); g.nodes.len()];
    let mut along_flow = 0.0;
    for (l, o) in order.iter().enumerate() {
        let mut c = snap((widest - span[l]) * 0.5);
        for &i in o {
            let (w, h) = sizes[i];
            at[i] = match dir {
                Direction::Lr => Rect { x: snap(along_flow), y: c, w, h },
                Direction::Tb => Rect { x: c, y: snap(along_flow), w, h },
            };
            c = snap(c + across(sizes[i]) + m.spacing);
        }
        along_flow += depth[l] + m.gap;
    }
    g.nodes
        .iter()
        .zip(at)
        .map(|(n, rect)| {
            let (ins, outs) = pins(n, rect, dir, m);
            Placed { rect, ins, outs }
        })
        .collect()
}

/// The nodes in reading order (Tab's): layer after layer along the flow, across each layer from its start.
pub fn order(placed: &[Placed], dir: Direction) -> Vec<usize> {
    let key = |i: usize| {
        let r = placed[i].rect;
        if dir == Direction::Lr { (r.x, r.y) } else { (r.y, r.x) }
    };
    let mut o: Vec<usize> = (0..placed.len()).collect();
    o.sort_by(|&a, &b| key(a).0.total_cmp(&key(b).0).then(key(a).1.total_cmp(&key(b).1)));
    o
}

/// what the laid out nodes cover together
pub fn bounds(placed: &[Placed]) -> Rect {
    bounds_of(placed.iter().map(|p| p.rect))
}

/// what some rectangles cover together
pub fn bounds_of(rects: impl IntoIterator<Item = Rect>) -> Rect {
    let mut it = rects.into_iter();
    let Some(first) = it.next() else { return Rect::default() };
    let (x0, y0, x1, y1) = it.fold((first.x, first.y, first.x + first.w, first.y + first.h), |o, r| (o.0.min(r.x), o.1.min(r.y), o.2.max(r.x + r.w), o.3.max(r.y + r.h)));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// how many places along a wire a value tag is tried at, each side of the middle
const TAG_TRIES: usize = 64;

/// the widths a value tag is tried at, as shares of its full width: a gap between two layers can be narrower than the text
const TAG_SHARES: [f32; 5] = [1.0, 0.75, 0.55, 0.4, 0.3];

/// Where a wire's value tag (`size`, centred on its spot) goes: at the wire's middle, else the nearest point along the
/// wire where it overlaps no `avoid` rect (the node cards, `margin` wider). Where the full width fits nowhere, the tag
/// narrows (the caller cuts its text) before the next try; with no spot at all, it sits beside the wire's end, on the
/// side the signal comes from. The tag's rect.
pub fn tag_spot(wire: &Wire, size: (f32, f32), avoid: &[Rect], dir: Direction, margin: f32) -> Rect {
    let step = 0.5 / TAG_TRIES as f32;
    for share in TAG_SHARES {
        let w = size.0 * share;
        let at = |c: (f32, f32)| Rect { x: c.0 - w * 0.5, y: c.1 - size.1 * 0.5, w, h: size.1 };
        let clear = |r: Rect| avoid.iter().all(|a| !(r.x < a.x + a.w + margin && a.x - margin < r.x + r.w && r.y < a.y + a.h + margin && a.y - margin < r.y + r.h));
        for k in 0..=TAG_TRIES {
            for t in [0.5 + step * k as f32, 0.5 - step * k as f32] {
                let r = at(wire.at(t));
                if clear(r) {
                    return r;
                }
            }
        }
    }
    let (end, w) = (wire.c[3], size.0 * TAG_SHARES[TAG_SHARES.len() - 1]);
    match dir {
        Direction::Lr => Rect { x: end.0 - w - margin, y: end.1 - size.1 * 0.5, w, h: size.1 },
        Direction::Tb => Rect { x: end.0 - w * 0.5, y: end.1 - size.1 - margin, w, h: size.1 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, ins: &[Signal], outs: &[Signal]) -> Node {
        let pins = |s: &[Signal]| s.iter().map(|s| Pin::new(s.word(), *s)).collect();
        Node { id: id.into(), title: id.into(), ins: pins(ins), outs: pins(outs), ..Node::default() }
    }

    fn link(from: (usize, usize), to: (usize, usize), signal: Signal) -> Link {
        Link::new(from, to, signal)
    }

    /// reader â†’ row â†’ two cells, the row also â†’ sends
    fn row() -> Graph {
        use Signal::*;
        Graph {
            nodes: vec![node("reader", &[], &[Event]), node("row", &[Event, Clock], &[Event]), node("left", &[Event, Control], &[Audio]), node("bells", &[Event, Far], &[Audio, Picture]), node("clock", &[], &[Clock, Clock])],
            links: vec![link((0, 0), (1, 0), Event), link((4, 1), (1, 1), Clock), link((1, 0), (2, 0), Event), link((1, 0), (3, 0), Event), link((2, 0), (3, 1), Far)],
        }
    }

    #[test]
    fn a_node_sits_one_layer_after_the_last_that_feeds_it() {
        assert_eq!(row().layers(), [0, 1, 2, 3, 0]);
        let mut looped = row();
        looped.links.push(link((3, 0), (1, 0), Signal::Event));
        let l = looped.layers();
        assert!(l.iter().all(|&x| x < looped.nodes.len()), "a loop ends: {l:?}");
    }

    #[test]
    fn left_to_right_puts_inlets_on_the_left_edge_and_outlets_on_the_right() {
        let m = Metrics::of(16.0);
        let p = layout(&row(), Direction::Lr, &m);
        for (n, at) in row().nodes.iter().zip(&p) {
            assert_eq!(at.ins.len(), n.ins.len());
            assert!(at.ins.iter().all(|q| q.0 == at.rect.x && q.1 > at.rect.y && q.1 < at.rect.y + at.rect.h));
            assert!(at.outs.iter().all(|q| q.0 == at.rect.x + at.rect.w));
        }
        // each layer to the right of the one before
        assert!(p[0].rect.x < p[1].rect.x && p[1].rect.x < p[2].rect.x && p[2].rect.x < p[3].rect.x);
        assert_eq!(p[0].rect.x, p[4].rect.x, "the clock beside the reader");
    }

    #[test]
    fn top_to_bottom_puts_inlets_on_top_and_outlets_below() {
        let m = Metrics::of(16.0);
        let p = layout(&row(), Direction::Tb, &m);
        for at in &p {
            assert!(at.ins.iter().all(|q| q.1 == at.rect.y && q.0 > at.rect.x && q.0 < at.rect.x + at.rect.w));
            assert!(at.outs.iter().all(|q| q.1 == at.rect.y + at.rect.h));
        }
        assert!(p[0].rect.y < p[1].rect.y && p[1].rect.y < p[2].rect.y);
    }

    #[test]
    fn every_node_is_on_the_grid_and_none_overlaps() {
        let m = Metrics::of(16.0);
        for dir in [Direction::Lr, Direction::Tb] {
            let p = layout(&row(), dir, &m);
            for (i, a) in p.iter().enumerate() {
                for v in [a.rect.x, a.rect.y, a.rect.w, a.rect.h] {
                    assert_eq!(v % m.unit, 0.0, "{dir:?} {v}");
                }
                for b in &p[i + 1..] {
                    let (a, b) = (a.rect, b.rect);
                    assert!(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y, "{dir:?} {a:?} {b:?}");
                }
            }
        }
    }

    #[test]
    fn a_collapsed_node_has_one_pin_per_signal_and_its_links_meet_there() {
        let mut g = row();
        g.nodes[4].collapsed = true;
        g.nodes[3].collapsed = true;
        let m = Metrics::of(16.0);
        let p = layout(&g, Direction::Lr, &m);
        assert_eq!(p[4].outs[0], p[4].outs[1], "two clock outlets: one pin");
        assert_ne!(p[3].outs[0], p[3].outs[1], "audio and picture: two");
        assert!(p[3].rect.h < layout(&row(), Direction::Lr, &m)[3].rect.h);
        assert_eq!(Node::merged(&g.nodes[3].outs), [Signal::Audio, Signal::Picture]);
    }

    #[test]
    fn what_feeds_a_layer_orders_it() {
        use Signal::*;
        // b is fed by the second of two sources, a by the first: a above b whatever the graph's order
        let g = Graph { nodes: vec![node("s1", &[], &[Event]), node("s2", &[], &[Event]), node("b", &[Event], &[]), node("a", &[Event], &[])], links: vec![link((1, 0), (2, 0), Event), link((0, 0), (3, 0), Event)] };
        let p = layout(&g, Direction::Lr, &Metrics::of(16.0));
        assert!(p[3].rect.y < p[2].rect.y);
    }

    #[test]
    fn a_fold_stands_for_its_members_and_keeps_the_wires_that_cross_it() {
        let g = row();
        // left and bells folded: the row's two event wires reach the pill, the far wire between them goes
        let f = g.fold(&[2, 3], Node { id: "group:a".into(), title: "a".into(), ..Node::default() });
        assert_eq!(f.nodes.len(), 4);
        let pill = f.node("group:a").unwrap();
        assert!(f.nodes[pill].collapsed);
        assert_eq!(f.nodes[pill].ins.len(), 2, "one pin per crossing wire");
        assert!(f.nodes[pill].outs.is_empty());
        assert_eq!(f.links.len(), 4);
        for l in &f.links {
            assert_eq!(f.nodes[l.to.0].ins[l.to.1].signal, l.signal);
            assert_eq!(f.nodes[l.from.0].outs[l.from.1].signal, l.signal);
        }
        let p = layout(&f, Direction::Lr, &Metrics::of(16.0));
        assert_eq!(p[pill].ins[0], p[pill].ins[1], "two event wires meet at one merged pin");
    }

    #[test]
    fn a_layer_keeps_a_groups_nodes_together() {
        use Signal::*;
        let mut g = Graph { nodes: vec![node("s", &[], &[Event]), node("a", &[Event], &[]), node("b", &[Event], &[]), node("c", &[Event], &[])], links: vec![link((0, 0), (1, 0), Event), link((0, 0), (2, 0), Event), link((0, 0), (3, 0), Event)] };
        g.nodes[1].group = Some(0);
        g.nodes[3].group = Some(0);
        let p = layout(&g, Direction::Lr, &Metrics::of(16.0));
        assert!(p[1].rect.y < p[3].rect.y && p[3].rect.y < p[2].rect.y, "a, c, then b");
    }

    #[test]
    fn an_open_node_with_a_picture_has_room_for_its_preview() {
        let mut g = row();
        g.nodes[2].picture = Some("scene:nave".into());
        g.nodes[3].picture = Some("scene:bells".into());
        g.nodes[3].collapsed = true;
        let m = Metrics { preview: 5.0 * 16.0, ..Metrics::of(16.0) };
        let (plain, shown) = (layout(&g, Direction::Lr, &Metrics::of(16.0)), layout(&g, Direction::Lr, &m));
        assert_eq!(shown[2].rect.h, plain[2].rect.h + 80.0);
        assert_eq!(shown[3].rect.h, plain[3].rect.h, "a folded node shows no preview");
        assert_eq!(shown[1].rect.h, plain[1].rect.h, "nor one without a picture");
    }

    #[test]
    fn tab_reads_layer_by_layer() {
        let p = layout(&row(), Direction::Lr, &Metrics::of(16.0));
        let o = order(&p, Direction::Lr);
        assert_eq!(&o[2..], [1, 2, 3], "the row, then the cells it fires");
        assert!(o[..2].contains(&0) && o[..2].contains(&4), "the sources first");
    }

    #[test]
    fn the_bounds_hold_every_node() {
        let p = layout(&row(), Direction::Lr, &Metrics::of(16.0));
        let b = bounds(&p);
        assert!(p.iter().all(|q| q.rect.x >= b.x && q.rect.y >= b.y && q.rect.x + q.rect.w <= b.x + b.w && q.rect.y + q.rect.h <= b.y + b.h));
        assert_eq!(bounds(&[]), Rect::default());
    }

    #[test]
    fn a_value_tag_moves_along_its_wire_off_the_nodes() {
        let w = Wire::hang((0.0, 0.0), (300.0, 0.0), 16.0);
        let size = (40.0, 16.0);
        let mid = w.at(0.5);
        let free = tag_spot(&w, size, &[], Direction::Lr, 4.0);
        assert_eq!((free.x + free.w * 0.5, free.y + free.h * 0.5), mid, "free: the middle");
        let on = Rect { x: mid.0 - 30.0, y: mid.1 - 30.0, w: 60.0, h: 60.0 };
        let at = tag_spot(&w, size, &[on], Direction::Lr, 4.0);
        assert_eq!(at.w, size.0, "room along the wire: whole");
        assert!(at.x + at.w <= on.x - 4.0 || at.x >= on.x + on.w + 4.0, "moved clear of the node: {at:?}");
        let (a, b) = (Rect { x: -50.0, y: -50.0, w: 120.0, h: 100.0 }, Rect { x: 110.0, y: -50.0, w: 300.0, h: 100.0 });
        let narrow = tag_spot(&w, size, &[a, b], Direction::Lr, 4.0);
        assert!(narrow.w < size.0 && narrow.x >= a.x + a.w + 4.0 && narrow.x + narrow.w <= b.x - 4.0, "a gap too small: the tag narrows {narrow:?}");
        let all = Rect { x: -50.0, y: -50.0, w: 500.0, h: 500.0 };
        let end = tag_spot(&w, size, &[all], Direction::Lr, 4.0);
        assert_eq!((end.x + end.w + 4.0, end.y + end.h * 0.5), (300.0, 0.0), "no spot: beside the end");
        let end = tag_spot(&w, size, &[all], Direction::Tb, 4.0);
        assert_eq!((end.x + end.w * 0.5, end.y + end.h + 4.0), (300.0, 0.0));
    }
}
