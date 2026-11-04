//! The module event host (#342, research/ABI-V2.md §1.4): deck-events events into a module's event inputs at their
//! frames, a module's event outputs back out as deck-events events – both ways inside the block, no thread.
//!
//! - **In from the world.** An event input listens to the names its descriptor declares: one route per name to one ring
//!   per port (the platform's dispatcher fans the deck's events out to it, like a host's `listen`). Each block the host
//!   places the due events on their frame (a beat on the block's tempo map, seconds on the audio timebase, a trigger at
//!   frame 0); one already past is late – frame 0, counted (`InHdr.late`, ModuleStats); one still ahead waits.
//! - **Notes.** `music.note` / `midi.note` become `NOTE_ON` with a host-assigned id, and a note with a duration gets its
//!   `NOTE_OFF` from the host at its end – a module never counts beats to end a note. Its pitch both ways: the Hz or the
//!   key the source gave, the other filled from the deck's tuning (`deck_events::Tuning`; `Seams::tuning`). A velocity
//!   the source lacks is 1.
//! - **Module to module.** An event output reaches every module event input that takes one of its names (by name, the
//!   props by key): directly, one block late at the same frame (`at + QUANTUM`), like a control output – so the order the
//!   nodes run in never changes the sound. A note's length is expanded into its off here; ids are re-tagged per link.
//! - **Out to the world.** Every event output also writes deck-events events into an outbox ring the platform drains into
//!   its dispatcher (`Events::take_outbox`): a named event with its props, a note with `dur` (a `len` > 0 at once; a note
//!   held until its `NOTE_OFF` when the off comes – one event with its true start and length), stamped at their true time.
//! - **Restarts.** The notes an input has delivered and not yet ended are kept; when a show swaps a rebuilt instance in,
//!   they are sent again at frame 0 of its first block (ABI-V2 §6: "the event host re-sends the notes still held").
//!
//! Everything is allocated at build; a block pops rings and writes preallocated lists (an event from the world that
//! carries heap data – an id, a source – frees it here, as the in-block hosts do).
use super::dispatch::Events;
use super::module::{ModuleInstance, Prepared};
use super::QUANTUM;
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_events::{Event, Match, Names, Route, Sym, Time, Tuning};
use deck_module_host::{Ev, EvKind};
use deck_space::graph::Graph;

/// records a ring holds (a link's, the world's, the outbox's)
const RING: usize = 512;
/// events an input keeps for later blocks (starts ahead, the offs of expanded notes)
const PENDING: usize = 512;
/// notes an input keeps as held (re-sent after a restart); notes an output keeps open for the world's `dur` form
const HELD: usize = 64;
/// no such name / prop slot on the other side
const NONE: u16 = u16::MAX;
const NO_SLOT: u8 = u8::MAX;
/// the names a note comes as: the generators' `music.note`, the MIDI adapter's `midi.note`
const NOTE_NAMES: [&str; 2] = ["music.note", "midi.note"];

/// the prop keys of a note
#[derive(Clone, Copy)]
struct Keys {
    pitch: Sym,
    vel: Sym,
    glide: Sym,
    key: Sym,
    /// the MIDI adapter's key prop
    note: Sym,
}

/// An output linked to a module's input: its records go one block late, names and props mapped by name.
struct Link {
    ring: rtrb::Producer<(u64, Ev)>,
    /// the output's name index → the input's (NONE: the input does not take it)
    names: Vec<u16>,
    /// the output's prop slot → the input's
    props: [u8; 6],
    /// both sides take notes
    notes: bool,
    /// the input's note ids from this link: `tag | (id & 0xff_ffff)`
    tag: u32,
}

struct In {
    port: usize,
    names: Vec<Sym>,
    props: Vec<Sym>,
    /// the index of the port's note name, if it takes notes
    note: Option<u16>,
    world: Option<rtrb::Consumer<Event>>,
    links: Vec<rtrb::Consumer<(u64, Ev)>>,
    /// (absolute frame, the record) not due yet, in arrival order
    pending: Vec<(u64, Ev)>,
    /// the notes delivered and not ended
    held: Vec<Ev>,
    next_id: u32,
}

struct Out {
    port: usize,
    names: Vec<Sym>,
    props: Vec<Sym>,
    note: Option<u16>,
    links: Vec<Link>,
    world: rtrb::Producer<Event>,
    /// notes on without a length, waiting for their off: (id, the on's frame, the on)
    open: Vec<(u32, u64, Ev)>,
}

/// One module node's event ports.
pub(crate) struct Ports {
    ins: Vec<In>,
    outs: Vec<Out>,
    scratch: Vec<Ev>,
    keys: Keys,
    tuning: Tuning,
}

/// Wires the event ports of every prepared module of `graph`: routes and rings from the world, the links between
/// modules, an outbox per output; the ports go into each `Prepared`.
pub(crate) fn wire(graph: &Graph, prepared: &mut [Option<Prepared>], ev: &mut Events, tuning: Tuning) -> Result<()> {
    let keys = {
        let mut key = |s: &str| ev.names.key(s).map_err(|e| anyhow!("{s}: {e:?}"));
        Keys { pitch: key("pitch")?, vel: key("vel")?, glide: key("glide")?, key: key("key")?, note: key("note")? }
    };
    let mut all: Vec<Option<Ports>> = (0..prepared.len()).map(|_| None).collect();
    for (i, p) in prepared.iter().enumerate() {
        let Some(p) = p else { continue };
        let l = p.layout();
        if l.events_in.is_empty() && l.events_out.is_empty() {
            continue;
        }
        let at = |port: &str| format!("node '{}': event port '{port}'", graph.nodes[i].id);
        let mut ins = Vec::new();
        for (port, e) in l.events_in.iter().enumerate() {
            let (names, note, props) = intern(&mut ev.names, &e.names, &e.props, &at(&e.name))?;
            let world = (!names.is_empty()).then(|| {
                let (tx, rx) = rtrb::RingBuffer::new(RING);
                for &s in &names {
                    ev.routes.push(Route { on: Match::Name(s), to: ev.outlets.len() });
                }
                ev.outlets.push(tx);
                rx
            });
            ins.push(In { port, names, props, note, world, links: Vec::new(), pending: Vec::with_capacity(PENDING), held: Vec::with_capacity(HELD), next_id: 1 });
        }
        let mut outs = Vec::new();
        for (port, e) in l.events_out.iter().enumerate() {
            let (names, note, props) = intern(&mut ev.names, &e.names, &e.props, &at(&e.name))?;
            let (tx, rx) = rtrb::RingBuffer::new(RING);
            ev.outbox.push(rx);
            outs.push(Out { port, names, props, note, links: Vec::new(), world: tx, open: Vec::with_capacity(HELD) });
        }
        all[i] = Some(Ports { ins, outs, scratch: Vec::with_capacity(PENDING + HELD), keys, tuning });
    }
    // the links: every output to every input that takes one of its names (in node and port order – deterministic)
    let mut specs = Vec::new();
    for (a, pa) in all.iter().enumerate() {
        let Some(pa) = pa else { continue };
        for (o, out) in pa.outs.iter().enumerate() {
            for (b, pb) in all.iter().enumerate() {
                let Some(pb) = pb else { continue };
                for (k, inp) in pb.ins.iter().enumerate() {
                    if out.names.iter().any(|n| inp.names.contains(n)) {
                        let names = out.names.iter().map(|n| inp.names.iter().position(|m| m == n).map_or(NONE, |x| x as u16)).collect::<Vec<_>>();
                        let mut props = [NO_SLOT; 6];
                        for (s, key) in out.props.iter().enumerate().take(6) {
                            if let Some(t) = inp.props.iter().position(|k| k == key) {
                                props[s] = t as u8;
                            }
                        }
                        specs.push((a, o, b, k, names, props, out.note.is_some() && inp.note.is_some()));
                    }
                }
            }
        }
    }
    for (n, (a, o, b, k, names, props, notes)) in specs.into_iter().enumerate() {
        let (tx, rx) = rtrb::RingBuffer::new(RING);
        let tag = ((n as u32 + 1) & 0xff) << 24;
        all[a].as_mut().expect("wired above").outs[o].links.push(Link { ring: tx, names, props, notes, tag });
        all[b].as_mut().expect("wired above").ins[k].links.push(rx);
    }
    for (p, ports) in prepared.iter_mut().zip(all) {
        if let Some(p) = p {
            p.ports = ports;
        }
    }
    Ok(())
}

/// a port's names (and which is its note), its prop keys – interned in the build's dictionary
fn intern(dict: &mut Names, names: &[String], props: &[String], at: &str) -> Result<(Vec<Sym>, Option<u16>, Vec<Sym>)> {
    let syms = names.iter().map(|n| dict.event(n).map_err(|e| anyhow!("{at}: name '{n}': {e:?}"))).collect::<Result<Vec<_>>>()?;
    let note = names.iter().position(|n| NOTE_NAMES.contains(&n.as_str())).map(|k| k as u16);
    let keys = props.iter().map(|k| dict.key(k).map_err(|e| anyhow!("{at}: prop '{k}': {e:?}"))).collect::<Result<Vec<_>>>()?;
    Ok((syms, note, keys))
}

/// a length in beats as frames on `map` (its tempo even while stopped)
fn beats_to_frames(map: &TempoMap, beats: f32) -> u64 {
    if map.bpm > 0.0 && beats > 0.0 {
        (f64::from(beats) * 60.0 * map.rate / map.bpm).round() as u64
    } else {
        0
    }
}

/// where an absolute frame lies on the deck's timeline: on the beat while the tempo runs, else on the audio timebase
fn time_of(map: &TempoMap, frame: u64) -> Time {
    if map.running {
        Time::Beats(map.beat_at(frame as f64))
    } else {
        Time::Seconds(frame as f64 / map.rate)
    }
}

impl In {
    fn queue(&mut self, at: u64, ev: Ev) {
        if self.pending.len() < self.pending.capacity() {
            self.pending.push((at, ev));
        }
    }

    /// a world event (deck-events) as records on this port
    fn take(&mut self, e: &Event, map: &TempoMap, start: f64, keys: &Keys, tuning: &Tuning) {
        let Some(idx) = self.names.iter().position(|s| *s == e.name) else { return };
        let at = e.frame(map, start).max(0.0).round() as u64;
        if Some(idx as u16) == self.note {
            let get = |k: Sym| e.props.get(k).unwrap_or(f32::NAN);
            let key = if get(keys.key).is_nan() { get(keys.note) } else { get(keys.key) };
            let (hz, key) = tuning.complete(get(keys.pitch), key);
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1) & 0x00ff_ffff;
            let mut on = Ev::note_on(0, id, hz, key, e.props.get(keys.vel).unwrap_or(1.0));
            on.v[2] = get(keys.glide);
            on.port = self.port as u8;
            self.queue(at, on);
            let dur = e.dur_frames(map);
            if dur > 0.0 {
                let mut off = Ev::note_off(0, id);
                off.port = self.port as u8;
                self.queue(at + dur.round() as u64, off);
            }
        } else {
            let mut r = Ev::event(0, idx as u16);
            r.port = self.port as u8;
            for (slot, k) in self.props.iter().enumerate().take(6) {
                r.v[slot] = e.props.get(*k).unwrap_or(f32::NAN);
            }
            self.queue(at, r);
        }
    }

    /// the notes delivered and not yet ended
    fn track(&mut self, ev: &Ev) {
        match ev.kind() {
            Some(EvKind::NoteOn) => {
                if self.held.len() < self.held.capacity() {
                    self.held.push(*ev);
                }
            }
            Some(EvKind::NoteOff) => {
                if let Some(k) = self.held.iter().position(|h| h.id == ev.id) {
                    self.held.remove(k);
                }
            }
            Some(EvKind::Reset) => self.held.clear(),
            _ => {}
        }
    }
}

impl Ports {
    /// Before a call over [start, start + n): every input port's due records into the instance (a swap first gets the
    /// held notes again at frame 0).
    pub(crate) fn deliver(&mut self, inst: &mut dyn ModuleInstance, map: &TempoMap, start: f64, n: usize) {
        let (from, to) = (start.max(0.0) as u64, start.max(0.0) as u64 + n as u64);
        let swapping = inst.swapping();
        for inp in &mut self.ins {
            if let Some(mut w) = inp.world.take() {
                while let Ok(e) = w.pop() {
                    inp.take(&e, map, start, &self.keys, &self.tuning);
                }
                inp.world = Some(w);
            }
            for k in 0..inp.links.len() {
                while let Ok((at, ev)) = inp.links[k].pop() {
                    inp.queue(at, ev);
                }
            }
            self.scratch.clear();
            if swapping {
                for h in &inp.held {
                    self.scratch.push(Ev { frame: 0, ..*h });
                }
            }
            let mut late = 0;
            let mut keep = 0;
            for i in 0..inp.pending.len() {
                let (at, mut ev) = inp.pending[i];
                if at < to {
                    ev.frame = at.saturating_sub(from) as u32;
                    late += u32::from(at < from);
                    inp.track(&ev);
                    if self.scratch.len() < self.scratch.capacity() {
                        self.scratch.push(ev);
                    }
                } else {
                    inp.pending[keep] = inp.pending[i];
                    keep += 1;
                }
            }
            inp.pending.truncate(keep);
            inst.set_events_in(inp.port, &self.scratch);
            if late > 0 {
                inst.add_late(late);
            }
        }
    }

    /// After a call over [start, start + n) that `ran`: every output's records to the linked inputs (one block late)
    /// and to the world (the outbox).
    pub(crate) fn collect(&mut self, inst: &dyn ModuleInstance, ran: bool, map: &TempoMap, start: f64, n: usize) {
        if !ran {
            return;
        }
        let from = start.max(0.0) as u64;
        for out in &mut self.outs {
            for e in inst.events_out(out.port) {
                let mut e = *e;
                if e.kind() == Some(EvKind::NoteOn) {
                    (e.v[0], e.v[3]) = self.tuning.complete(e.v[0], e.v[3]);
                }
                let at = from + u64::from(e.frame.min(n.saturating_sub(1) as u32));
                for l in &mut out.links {
                    l.send(&e, at + QUANTUM as u64, map);
                }
                out.world(&e, at, map, &self.keys);
            }
        }
    }
}

impl Link {
    fn send(&mut self, e: &Ev, at: u64, map: &TempoMap) {
        match e.kind() {
            Some(EvKind::Event) => {
                let Some(&name) = self.names.get(usize::from(e.name)).filter(|&&n| n != NONE) else { return };
                let mut r = Ev::event(0, name);
                for (s, &t) in self.props.iter().enumerate() {
                    if t != NO_SLOT {
                        r.v[usize::from(t)] = e.v[s];
                    }
                }
                let _ = self.ring.push((at, r));
            }
            Some(_) if self.notes => {
                let mut r = *e;
                r.id = self.tag | (e.id & 0x00ff_ffff);
                r.len = 0.0;
                let _ = self.ring.push((at, r));
                if e.kind() == Some(EvKind::NoteOn) && e.len > 0.0 {
                    let _ = self.ring.push((at + beats_to_frames(map, e.len), Ev::note_off(0, r.id)));
                }
            }
            _ => {}
        }
    }
}

impl Out {
    /// a note as deck-events' event from `on` (at frame `at`) lasting `dur`
    fn note(&self, on: &Ev, at: u64, dur: Option<Time>, map: &TempoMap, keys: &Keys) -> Option<Event> {
        let name = *self.names.get(usize::from(self.note?))?;
        let mut e = Event::new(name, time_of(map, at));
        e.dur = dur;
        for (k, v) in [(keys.pitch, on.v[0]), (keys.vel, on.v[1]), (keys.glide, on.v[2]), (keys.key, on.v[3])] {
            if !v.is_nan() {
                e.props.set(k, v);
            }
        }
        Some(e)
    }

    /// the length between two frames on the deck's timeline
    fn span(map: &TempoMap, from: u64, to: u64) -> Time {
        if map.running {
            Time::Beats(map.beat_at(to as f64) - map.beat_at(from as f64))
        } else {
            Time::Seconds(to.saturating_sub(from) as f64 / map.rate)
        }
    }

    fn world(&mut self, e: &Ev, at: u64, map: &TempoMap, keys: &Keys) {
        match e.kind() {
            Some(EvKind::Event) => {
                let Some(&name) = self.names.get(usize::from(e.name)) else { return };
                let mut ev = Event::new(name, time_of(map, at));
                for (k, v) in self.props.iter().zip(e.v) {
                    if !v.is_nan() {
                        ev.props.set(*k, v);
                    }
                }
                let _ = self.world.push(ev);
            }
            Some(EvKind::NoteOn) if e.len > 0.0 => {
                if let Some(ev) = self.note(e, at, Some(Time::Beats(f64::from(e.len))), map, keys) {
                    let _ = self.world.push(ev);
                }
            }
            Some(EvKind::NoteOn) => {
                if self.open.len() < self.open.capacity() {
                    self.open.push((e.id, at, *e));
                }
            }
            Some(EvKind::NoteOff) => {
                if let Some(k) = self.open.iter().position(|o| o.0 == e.id) {
                    let (_, on_at, on) = self.open.remove(k);
                    if let Some(ev) = self.note(&on, on_at, Some(Out::span(map, on_at, at)), map, keys) {
                        let _ = self.world.push(ev);
                    }
                }
            }
            Some(EvKind::Reset) => {
                while let Some((_, on_at, on)) = self.open.pop() {
                    if let Some(ev) = self.note(&on, on_at, Some(Out::span(map, on_at, at)), map, keys) {
                        let _ = self.world.push(ev);
                    }
                }
            }
            // a note expression has no deck-events form yet
            _ => {}
        }
    }
}
