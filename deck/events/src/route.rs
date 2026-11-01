//! The routing table (research/EVENTS.md §15 "a receiver subscribes to names or whole domains", §16 the dispatcher):
//! receivers (outlets – a host's input, later a frame-thread consumer) listen to an event name (`midi.note`) or a
//! whole domain (`story`). Built once from the name dictionary when a patch loads – every name's receivers resolved
//! up front – so routing an event is one array lookup and a clone per receiver: no strings, no allocation (an event
//! without children clones without allocating). Immutable: a patch change builds a new table and swaps it whole.
//! Platform-free: the stage's dispatcher thread uses it; the browser's worker can too.
use crate::event::Event;
use crate::graph::{Stats, MAX_HOPS};
use crate::names::{Names, Sym};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// one event name
    Name(Sym),
    /// every event whose name starts with this domain (its word interned as a key: `names.key("story")`)
    Domain(Sym),
}

/// `on` → outlet `to`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub on: Match,
    pub to: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Table {
    /// per name (Sym index): its receivers' range in `targets`
    ranges: Vec<(u32, u32)>,
    targets: Vec<u32>,
    outlets: usize,
}

impl Table {
    /// every name interned in `names` so far resolved to its outlets (each at most once, in route order); a name
    /// interned later has no receivers – names are interned at load
    pub fn new(names: &Names, routes: &[Route], outlets: usize) -> Table {
        let mut ranges = Vec::with_capacity(names.len());
        let mut targets = Vec::new();
        for i in 0..names.len() {
            let sym = Sym(i as u32);
            let domain = names.domain(sym).and_then(|d| names.get(d));
            let from = targets.len() as u32;
            for r in routes.iter().filter(|r| r.to < outlets) {
                let hit = match r.on {
                    Match::Name(n) => n == sym,
                    Match::Domain(d) => Some(d) == domain,
                };
                if hit && !targets[from as usize..].contains(&(r.to as u32)) {
                    targets.push(r.to as u32);
                }
            }
            ranges.push((from, targets.len() as u32));
        }
        Table { ranges, targets, outlets }
    }

    /// the outlets listening to `name`
    pub fn targets(&self, name: Sym) -> &[u32] {
        match self.ranges.get(name.0 as usize) {
            Some(&(a, b)) => &self.targets[a as usize..b as usize],
            None => &[],
        }
    }

    pub fn outlets(&self) -> usize {
        self.outlets
    }

    /// Fan `e` out: `push(outlet, event)` per receiver (false = that outlet is full: counted in `capped`). Refused
    /// and counted: a compound event (`refused` – send a reference, §16), one past MAX_HOPS (`hops`). Returns the
    /// receivers reached. Each copy counts one hop more, like an edge.
    pub fn route(&self, mut e: Event, stats: &mut Stats, mut push: impl FnMut(usize, Event) -> bool) -> usize {
        if !e.children.is_empty() {
            stats.refused += 1;
            return 0;
        }
        if e.hops >= MAX_HOPS {
            stats.hops += 1;
            return 0;
        }
        e.hops += 1;
        let to = self.targets(e.name);
        let mut reached = 0;
        for (k, &o) in to.iter().enumerate() {
            let copy = if k + 1 == to.len() { std::mem::replace(&mut e, Event::now(Sym(0))) } else { e.clone() };
            if push(o as usize, copy) {
                reached += 1;
            } else {
                stats.capped += 1;
            }
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_domains_reach_their_outlets_once() {
        let mut n = Names::new();
        let (note, cc, fx) = (n.event("midi.note").unwrap(), n.event("midi.cc").unwrap(), n.event("story.fx").unwrap());
        let midi = n.key("midi").unwrap();
        let routes = [
            Route { on: Match::Name(note), to: 0 },
            Route { on: Match::Domain(midi), to: 1 },
            Route { on: Match::Domain(midi), to: 0 },
            Route { on: Match::Name(fx), to: 7 },
        ];
        let t = Table::new(&n, &routes, 2);
        assert_eq!(t.targets(note), &[0, 1], "outlet 0 once although two routes match");
        assert_eq!(t.targets(cc), &[1, 0]);
        assert!(t.targets(fx).is_empty(), "an outlet that doesn't exist is no receiver");
        assert!(t.targets(Sym(999)).is_empty(), "a name interned after the table");
    }

    #[test]
    fn routing_copies_counts_hops_and_refuses_compound_events() {
        let mut n = Names::new();
        let note = n.event("midi.note").unwrap();
        let pitch = n.key("pitch").unwrap();
        let t = Table::new(&n, &[Route { on: Match::Name(note), to: 0 }, Route { on: Match::Name(note), to: 1 }], 2);
        let mut stats = Stats::default();
        let mut got = Vec::new();
        assert_eq!(t.route(Event::now(note).with(pitch, 220.0), &mut stats, |o, e| {
            got.push((o, e.hops, e.props.get(pitch)));
            o == 0
        }), 1);
        assert_eq!(got, [(0, 1, Some(220.0)), (1, 1, Some(220.0))]);
        assert_eq!(stats.capped, 1, "outlet 1 was full");
        let compound = Event::now(note).containing(vec![Event::now(note)]);
        assert_eq!(t.route(compound, &mut stats, |_, _| true), 0);
        let mut tired = Event::now(note);
        tired.hops = MAX_HOPS;
        assert_eq!(t.route(tired, &mut stats, |_, _| true), 0);
        assert_eq!((stats.refused, stats.hops), (1, 1));
    }
}
