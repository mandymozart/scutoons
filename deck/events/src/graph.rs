//! The event graph (research/EVENTS.md §4, §15): generators joined output → input. Once per window `run` asks every
//! generator in topological order and hands each one's events to its consumers. Loops are possible and bounded:
//! - a loop without a **delayed** edge is refused when the graph is built (like graph.rs refuses cycles); a delayed
//!   edge delivers in the next window, so a loop runs at most once per window
//! - each edge carries at most EDGE_CAP events a window; the rest is dropped and counted (`Stats::capped`)
//! - every event counts its hops; past MAX_HOPS it is dropped and counted (`Stats::hops`) – this also ends loops that
//!   leave the deck and come back (through the relay)
//!
//! External triggers (a story event, a relay cue, a MIDI key – the adapters) come in with `inject` for the next run.
use crate::event::Event;
use crate::generator::{Generator, Inbox, Outbox, Span};
use std::fmt;

/// events an edge carries per window at most
pub const EDGE_CAP: usize = 256;
/// edges an event may cross
pub const MAX_HOPS: u8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub delayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    NoNode(usize),
    NoPort { node: usize, port: String, input: bool },
    /// nodes on a loop without a delayed edge
    Cycle(Vec<usize>),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NoNode(n) => write!(f, "no node {n}"),
            GraphError::NoPort { node, port, input } => {
                write!(f, "node {node} has no {} '{port}'", if *input { "input" } else { "output" })
            }
            GraphError::Cycle(nodes) => write!(f, "a loop without a delayed edge through nodes {nodes:?} – mark one edge delayed"),
        }
    }
}

impl std::error::Error for GraphError {}

/// dropped events since the last `take_stats` (the HUD shows them like dropouts)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// over an edge's or an output's cap
    pub capped: u64,
    /// past MAX_HOPS
    pub hops: u64,
}

#[derive(Default)]
pub struct EventGraph {
    nodes: Vec<Box<dyn Generator>>,
    edges: Vec<Edge>,
    order: Vec<usize>,
    dirty: bool,
    inboxes: Vec<Vec<Vec<Event>>>,
    outboxes: Vec<Outbox>,
    /// per edge: delayed events for this run, and the ones collected for the next
    pending: Vec<Vec<Event>>,
    next: Vec<Vec<Event>>,
    injected: Vec<(usize, usize, Event)>,
    stats: Stats,
}

impl EventGraph {
    pub fn new() -> EventGraph {
        EventGraph::default()
    }

    pub fn add(&mut self, g: impl Generator + 'static) -> usize {
        let p = g.ports();
        self.inboxes.push(vec![Vec::new(); p.ins.len()]);
        self.outboxes.push(Outbox::new(p.outs.len(), EDGE_CAP));
        self.nodes.push(Box::new(g));
        self.dirty = true;
        self.nodes.len() - 1
    }

    fn port(&self, node: usize, name: &str, input: bool) -> Result<usize, GraphError> {
        let g = self.nodes.get(node).ok_or(GraphError::NoNode(node))?;
        let p = g.ports();
        let list = if input { p.ins } else { p.outs };
        list.iter().position(|n| *n == name).ok_or(GraphError::NoPort { node, port: name.into(), input })
    }

    /// join `from`'s output to `to`'s input; `delayed` = delivered in the next window (a loop needs one)
    pub fn connect(&mut self, from: (usize, &str), to: (usize, &str), delayed: bool) -> Result<(), GraphError> {
        let f = self.port(from.0, from.1, false)?;
        let t = self.port(to.0, to.1, true)?;
        self.edges.push(Edge { from: (from.0, f), to: (to.0, t), delayed });
        self.pending.push(Vec::new());
        self.next.push(Vec::new());
        self.dirty = true;
        Ok(())
    }

    /// The order to run in (Kahn's algorithm over the edges that aren't delayed); refuses a loop without a delay.
    pub fn build(&mut self) -> Result<(), GraphError> {
        let n = self.nodes.len();
        let mut indeg = vec![0usize; n];
        for e in self.edges.iter().filter(|e| !e.delayed) {
            indeg[e.to.0] += 1;
        }
        let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
        ready.reverse();
        let mut order = Vec::with_capacity(n);
        while let Some(i) = ready.pop() {
            order.push(i);
            for e in self.edges.iter().filter(|e| !e.delayed && e.from.0 == i) {
                indeg[e.to.0] -= 1;
                if indeg[e.to.0] == 0 {
                    ready.push(e.to.0);
                }
            }
        }
        if order.len() < n {
            return Err(GraphError::Cycle((0..n).filter(|i| !order.contains(i)).collect()));
        }
        self.order = order;
        self.dirty = false;
        Ok(())
    }

    /// an external event (an adapter's) for `node`'s input in the next run
    pub fn inject(&mut self, node: usize, port: &str, e: Event) -> Result<(), GraphError> {
        let p = self.port(node, port, true)?;
        self.injected.push((node, p, e));
        Ok(())
    }

    /// One window: deliver the delayed and injected events, ask every generator in order, pass its events on.
    pub fn run(&mut self, span: &Span) -> Result<(), GraphError> {
        if self.dirty {
            self.build()?;
        }
        for ports in &mut self.inboxes {
            for p in ports {
                p.clear();
            }
        }
        for (i, evs) in self.pending.iter_mut().enumerate() {
            let (n, p) = self.edges[i].to;
            self.inboxes[n][p].append(evs);
        }
        for (n, p, e) in self.injected.drain(..) {
            self.inboxes[n][p].push(e);
        }
        for k in 0..self.order.len() {
            let n = self.order[k];
            for p in &mut self.inboxes[n] {
                p.sort_by(|a, b| a.frame(span.tempo, span.from_frame).total_cmp(&b.frame(span.tempo, span.from_frame)));
            }
            let out = &mut self.outboxes[n];
            out.clear();
            self.nodes[n].query(span, &Inbox { ports: &self.inboxes[n] }, out);
            self.stats.capped += std::mem::take(&mut out.dropped);
            for (ei, e) in self.edges.iter().enumerate().filter(|(_, e)| e.from.0 == n) {
                let mut sent = 0;
                for ev in self.outboxes[n].port(e.from.1) {
                    if ev.hops >= MAX_HOPS {
                        self.stats.hops += 1;
                        continue;
                    }
                    if sent == EDGE_CAP {
                        self.stats.capped += 1;
                        continue;
                    }
                    sent += 1;
                    let mut c = ev.clone();
                    c.hops += 1;
                    if e.delayed {
                        self.next[ei].push(c);
                    } else {
                        self.inboxes[e.to.0][e.to.1].push(c);
                    }
                }
            }
        }
        std::mem::swap(&mut self.pending, &mut self.next);
        Ok(())
    }

    /// what `node` emitted on `port` in the last run (a sink reads its consumer's view here)
    pub fn outputs(&self, node: usize, port: &str) -> Result<&[Event], GraphError> {
        let p = self.port(node, port, false)?;
        Ok(self.outboxes[node].port(p))
    }

    pub fn take_stats(&mut self) -> Stats {
        std::mem::take(&mut self.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::Ports;
    use crate::names::{Names, Sym};
    use deck_core::tempo::TempoMap;

    /// passes its input on, `copies` times each – a doubling loop's worst case
    struct Echo {
        copies: usize,
    }
    impl Generator for Echo {
        fn ports(&self) -> Ports {
            Ports { ins: &["in"], outs: &["out"] }
        }
        fn query(&mut self, _: &Span, ins: &Inbox, out: &mut Outbox) {
            for e in ins.port(0) {
                for _ in 0..self.copies {
                    out.push(0, e.clone());
                }
            }
        }
    }

    /// emits one trigger per window
    struct Tick(Sym);
    impl Generator for Tick {
        fn ports(&self) -> Ports {
            Ports { ins: &[], outs: &["out"] }
        }
        fn query(&mut self, _: &Span, _: &Inbox, out: &mut Outbox) {
            out.push(0, Event::now(self.0));
        }
    }

    fn tempo() -> TempoMap {
        TempoMap::new(120.0, 4, 48_000.0)
    }

    #[test]
    fn a_loop_without_a_delay_is_refused() {
        let mut g = EventGraph::new();
        let (a, b) = (g.add(Echo { copies: 1 }), g.add(Echo { copies: 1 }));
        g.connect((a, "out"), (b, "in"), false).unwrap();
        g.connect((b, "out"), (a, "in"), false).unwrap();
        assert_eq!(g.build(), Err(GraphError::Cycle(vec![a, b])));
        assert!(matches!(g.connect((a, "nope"), (b, "in"), false), Err(GraphError::NoPort { .. })));
    }

    #[test]
    fn a_delayed_loop_runs_once_a_window_and_its_growth_is_capped() {
        let mut names = Names::new();
        let ping = names.event("test.ping").unwrap();
        let mut g = EventGraph::new();
        let (a, b) = (g.add(Echo { copies: 2 }), g.add(Echo { copies: 1 }));
        g.connect((a, "out"), (b, "in"), false).unwrap();
        g.connect((b, "out"), (a, "in"), true).unwrap();
        g.inject(a, "in", Event::now(ping)).unwrap();
        let t = tempo();
        let mut sizes = Vec::new();
        for w in 0..12 {
            g.run(&Span::frames(&t, w as f64 * 256.0, (w + 1) as f64 * 256.0)).unwrap();
            sizes.push(g.outputs(b, "out").unwrap().len());
        }
        // doubling each window: 2, 4, 8 … until the cap holds it
        assert_eq!(&sizes[..4], &[2, 4, 8, 16]);
        assert!(sizes.iter().all(|&n| n <= EDGE_CAP), "{sizes:?}");
        let s = g.take_stats();
        assert!(s.capped > 0 || s.hops > 0, "the runaway is counted: {s:?}");
    }

    #[test]
    fn hops_end_a_loop_that_keeps_one_event_alive() {
        let mut names = Names::new();
        let ping = names.event("test.ping").unwrap();
        let mut g = EventGraph::new();
        let (a, b) = (g.add(Echo { copies: 1 }), g.add(Echo { copies: 1 }));
        g.connect((a, "out"), (b, "in"), false).unwrap();
        g.connect((b, "out"), (a, "in"), true).unwrap();
        g.inject(a, "in", Event::now(ping)).unwrap();
        let t = tempo();
        let mut alive = 0;
        for w in 0..40 {
            g.run(&Span::frames(&t, w as f64 * 256.0, (w + 1) as f64 * 256.0)).unwrap();
            alive += g.outputs(b, "out").unwrap().len();
        }
        assert_eq!(alive, MAX_HOPS as usize / 2, "two hops a round trip, dropped at MAX_HOPS");
        assert_eq!(g.take_stats().hops, 1);
    }

    #[test]
    fn events_flow_in_order_through_a_chain() {
        let mut names = Names::new();
        let tick = names.event("test.tick").unwrap();
        let mut g = EventGraph::new();
        // added consumer first: the order comes from the edges, not from the adding
        let echo = g.add(Echo { copies: 1 });
        let src = g.add(Tick(tick));
        g.connect((src, "out"), (echo, "in"), false).unwrap();
        let t = tempo();
        g.run(&Span::frames(&t, 0.0, 256.0)).unwrap();
        let out = g.outputs(echo, "out").unwrap();
        assert_eq!(out.len(), 1, "the source ran first, in the same window");
        assert_eq!((out[0].name, out[0].hops), (tick, 1));
    }
}
