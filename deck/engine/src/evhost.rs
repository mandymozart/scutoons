//! The module event host (#342, research/ABI-V2.md §1.4): deck-events events into a module's event inputs at their
//! frames, a module's event outputs back out as deck-events events – both ways inside the block, no thread.
//!
//! - **In from the world.** A `world` node (#430, deck-space's world.rs) holds its name filter and is cabled like a module:
//!   an event input with a cable from a MIDI world node gets one ring and one route per name of the filter it can take
//!   (the platform's dispatcher fans the deck's events out to it, like a host's `listen`). An event input with no cable
//!   hears nothing. Each block the host places the due events on their frame (a beat on the block's tempo map, seconds on
//!   the audio timebase, a trigger at frame 0); one already past is late – frame 0, counted (`InHdr.late`, ModuleStats);
//!   one still ahead waits. A `generator` world node runs its deck-events patch itself (walk.rs `Host`) and sends its
//!   notes down its cables like a module's outputs, so two generators stay apart.
//! - **Notes.** `music.note` / `midi.note` become `NOTE_ON` with a host-assigned id, and a note with a duration gets its
//!   `NOTE_OFF` from the host at its end – a module never counts beats to end a note. Its pitch both ways: the Hz or the
//!   key the source gave, the other filled from the deck's tuning (`deck_events::Tuning`; `Seams::tuning`). A velocity
//!   the source lacks is 1.
//! - **Module to module, by cable (#430).** An event output reaches exactly the inputs its cables name (`Graph::links` of
//!   type event, many to many): names and props mapped by name (a note is a note whatever name it comes as),
//!   one block late at the same frame (`at + QUANTUM`), like a control output – so the order the nodes run in never changes
//!   the sound. A note's length is expanded into its off here; ids are re-tagged per link. A module does not hear its own
//!   outputs (the graph refuses that cable); a loop through two modules is one block late per turn – the patch's to design.
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
use deck_events::rt::RtEvent;
use deck_module_host::{Ev, EvKind};
use deck_space::graph::{Graph, Kind, PortType};
use deck_space::world::{self, Source, NOTE_NAMES};

/// records a ring holds (a link's, the world's, the outbox's)
const RING: usize = 512;
/// events an input keeps for later blocks (starts ahead, the offs of expanded notes)
const PENDING: usize = 512;
/// notes an input keeps as held (re-sent after a restart); notes an output keeps open for the world's `dur` form
const HELD: usize = 64;
/// no such name / prop slot on the other side
const NONE: u16 = u16::MAX;
const NO_SLOT: u8 = u8::MAX;
/// the prop keys of a note
#[derive(Clone, Copy)]
struct Keys {
    pitch: Sym,
    vel: Sym,
    glide: Sym,
    key: Sym,
    /// the MIDI adapter's key prop
    note: Sym,
    /// the event names a note comes as
    notes: [Sym; 2],
}

/// An output (a module's, a generator world node's) cabled to a module's input: its records go one block late, names and
/// props mapped by name.
pub(crate) struct Link {
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

/// Wires the event ports of every prepared module of `graph` along its event cables: a ring and routes per input a MIDI
/// world node feeds, the links between modules and from a generator world node, an outbox per output; the ports go into
/// each `Prepared`, a generator's links into `Events` for its block (walk.rs).
pub(crate) fn wire(graph: &Graph, prepared: &mut [Option<Prepared>], ev: &mut Events, tuning: Tuning) -> Result<()> {
    let keys = {
        let mut key = |s: &str| ev.names.key(s).map_err(|e| anyhow!("{s}: {e:?}"));
        let (pitch, vel, glide, k, note) = (key("pitch")?, key("vel")?, key("glide")?, key("key")?, key("note")?);
        let mut event = |s: &str| ev.names.event(s).map_err(|e| anyhow!("{s}: {e:?}"));
        Keys { pitch, vel, glide, key: k, note, notes: [event(NOTE_NAMES[0])?, event(NOTE_NAMES[1])?] }
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
            ins.push(In { port, names, props, note, world: None, links: Vec::new(), pending: Vec::with_capacity(PENDING), held: Vec::with_capacity(HELD), next_id: 1 });
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
    // the cables (in the scene's order – deterministic)
    let mut world_rings: Vec<((usize, usize), usize)> = Vec::new();
    let mut specs = Vec::new();
    let mut generators: Vec<(usize, usize, usize, Vec<u16>)> = Vec::new();
    for l in graph.links.iter().filter(|l| l.ty == PortType::Event) {
        let (a, o) = l.from;
        let (b, kb) = l.to;
        let Some(pb) = all[b].as_ref() else { continue };
        let inputs = &prepared[b].as_ref().expect("a module of the graph").layout().events_in;
        let port = graph.nodes[b].kind.inputs()[kb].name;
        let k = inputs.iter().position(|e| e.name == port).ok_or_else(|| anyhow!("node '{}': no event input '{port}'", graph.nodes[b].id))?;
        if graph.nodes[a].kind == Kind::World {
            let w = world::World::of(&graph.nodes[a])?;
            match &w.source {
                Source::Midi => {
                    let outlet = match world_rings.iter().find(|(at, _)| *at == (b, k)) {
                        Some(&(_, t)) => t,
                        None => {
                            let (tx, rx) = rtrb::RingBuffer::new(RING);
                            let t = ev.outlets.len();
                            ev.outlets.push(tx);
                            all[b].as_mut().expect("wired above").ins[k].world = Some(rx);
                            world_rings.push(((b, k), t));
                            t
                        }
                    };
                    for name in &w.listen {
                        let s = ev.names.event(name).map_err(|e| anyhow!("{name}: {e:?}"))?;
                        let inp = &all[b].as_ref().expect("wired above").ins[k];
                        let takes = inp.names.contains(&s) || (keys.notes.contains(&s) && inp.note.is_some());
                        if takes && !ev.routes.iter().any(|r| r.on == Match::Name(s) && r.to == outlet) {
                            ev.routes.push(Route { on: Match::Name(s), to: outlet });
                        }
                    }
                }
                Source::Generator => {
                    // a generator's notes come straight down its cables; its one name maps onto the input's note
                    let note = pb.ins[k].note.ok_or_else(|| anyhow!("node '{}': a generator's cable needs a note input", graph.nodes[b].id))?;
                    generators.push((a, b, k, vec![note]));
                }
                // #432: a far place's events – nothing carries them into this deck yet (#442): the input hears nothing from it
                Source::Far(_) => {}
            }
            continue;
        }
        let Some(pa) = all[a].as_ref() else { continue };
        let out_port = graph.nodes[a].kind.outputs()[o].name;
        let o = prepared[a].as_ref().expect("a module of the graph").layout().events_out.iter().position(|e| e.name == out_port).ok_or_else(|| anyhow!("node '{}': no event output '{out_port}'", graph.nodes[a].id))?;
        let (out, inp) = (&pa.outs[o], &pb.ins[k]);
        let names = out.names.iter().map(|n| inp.names.iter().position(|m| m == n).map_or(NONE, |x| x as u16)).collect::<Vec<_>>();
        let mut props = [NO_SLOT; 6];
        for (s, key) in out.props.iter().enumerate().take(6) {
            if let Some(t) = inp.props.iter().position(|k| k == key) {
                props[s] = t as u8;
            }
        }
        specs.push((a, o, b, k, names, props, out.note.is_some() && inp.note.is_some()));
    }
    let tag = |n: usize| ((n as u32 + 1) & 0xff) << 24;
    let mut n = 0;
    for (a, o, b, k, names, props, notes) in specs {
        let (tx, rx) = rtrb::RingBuffer::new(RING);
        all[a].as_mut().expect("wired above").outs[o].links.push(Link { ring: tx, names, props, notes, tag: tag(n) });
        all[b].as_mut().expect("wired above").ins[k].links.push(rx);
        n += 1;
    }
    for (a, b, k, names) in generators {
        let (tx, rx) = rtrb::RingBuffer::new(RING);
        all[b].as_mut().expect("wired above").ins[k].links.push(rx);
        let link = Link { ring: tx, names, props: [NO_SLOT; 6], notes: true, tag: tag(n) };
        n += 1;
        match ev.generators.iter_mut().find(|(at, _)| *at == a) {
            Some((_, links)) => links.push(link),
            None => ev.generators.push((a, vec![link])),
        }
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

    /// a world event (deck-events) as records on this port (a note is a note whatever name it comes as)
    fn take(&mut self, e: &Event, map: &TempoMap, start: f64, keys: &Keys, tuning: &Tuning) {
        let idx = match self.names.iter().position(|s| *s == e.name) {
            Some(i) => i,
            None if keys.notes.contains(&e.name) => match self.note {
                Some(n) => usize::from(n),
                None => return,
            },
            None => return,
        };
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

/// A `generator` world node's way out (#430): its window's notes down its cables, one block late at their frame, a note's
/// off from its length – as a module's outputs go.
pub(crate) struct WorldOut {
    links: Vec<Link>,
    pitch: Sym,
    vel: Sym,
    glide: Sym,
    tuning: Tuning,
    next_id: u32,
}

impl WorldOut {
    /// None without a cable: the generator has nobody to play to
    pub(crate) fn new(links: Vec<Link>, names: &mut Names, tuning: Tuning) -> Result<Option<WorldOut>> {
        if links.is_empty() {
            return Ok(None);
        }
        let mut key = |s: &str| names.key(s).map_err(|e| anyhow!("{s}: {e:?}"));
        Ok(Some(WorldOut { links, pitch: key("pitch")?, vel: key("vel")?, glide: key("glide")?, tuning, next_id: 1 }))
    }

    /// a window's events (`from` = its first frame) sent on
    pub(crate) fn send(&mut self, events: &[RtEvent], from: u64) {
        for e in events {
            let get = |k: Sym| e.props.get(k).unwrap_or(f32::NAN);
            let (hz, key) = self.tuning.complete(get(self.pitch), f32::NAN);
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1) & 0x00ff_ffff;
            let vel = e.props.get(self.vel).unwrap_or(1.0);
            let mut on = Ev::note_on(0, id, hz, key, vel);
            on.v[2] = get(self.glide);
            let at = from + u64::from(e.frame) + QUANTUM as u64;
            for l in &mut self.links {
                l.note(on, at, u64::from(e.len));
            }
        }
    }
}

impl Link {
    /// a note on at `at` and its off `frames` later, under this link's ids
    fn note(&mut self, on: Ev, at: u64, frames: u64) {
        let mut r = on;
        r.id = self.tag | (on.id & 0x00ff_ffff);
        r.len = 0.0;
        let _ = self.ring.push((at, r));
        if frames > 0 {
            let _ = self.ring.push((at + frames, Ev::note_off(0, r.id)));
        }
    }

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
            Some(kind) if self.notes => {
                let frames = if kind == EvKind::NoteOn { beats_to_frames(map, e.len) } else { 0 };
                self.note(*e, at, frames);
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
