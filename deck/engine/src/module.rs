//! A rig module as a node (#280; ABI v2 #342, research/ABI-V2.md §1.3, §1.4, §3, §5): `module:<name>` in a scene.
//! - **Ports.** Its graph inputs are the module's audio inputs, control inputs and parameters (every parameter is also a
//!   control input – a link into `w.spread` sets it, an unlinked one holds the node's TOML value or the descriptor's
//!   default); its graph outputs are its audio outputs and – ABI v2 – its control outputs (deck-module-host's `Layout`).
//!   Audio crosses as channel slices of the engine's planar pool: as many channels as the cable carries, up to the port's.
//! - **Domains.** An `audio` module is a block of the audio half (`ModuleBlock`, `dm_process`); a `control` module is a
//!   stepper of the control half (`ControlModule`, `dm_control`, once per block after the control levels).
//! - **One block late.** A control output written in block k reaches its consumers at block k + 1 – a cable or a `path`
//!   target alike (deck-space binds a path to its input as a link): the module writes a `Param`, the control half copies
//!   it into the output's value slot at the next block's start. Unwritten = held. Events a module emits reach other
//!   modules one block late at the same frame (evhost.rs). So the order the nodes run in never changes the sound.
//! - **Faults.** Strict (tests, Play): a trap or an overrun silences the node – zeros from then on, one report. In a show
//!   (the platform opens its instances so) a trap restarts the instance: its rebuild goes to the platform's loader thread
//!   through the node's `Restarts` lane (the engine starts no thread) and comes back swapped in with a fade; the event
//!   host re-sends the notes still held at the swap.
//! - **Context.** Node params besides the module's own: `seed` (the module's seed – every node starts at 1, the node's
//!   `seed` changes it, owner 2026-11-03), `character`, `persona`, `mood` (strings), `vars` (a table of key → number).
//!   The live context (#409, control.rs `Command::Context`: the deck's mood, persona, the being's energy / touch /
//!   strain) replaces persona, mood and the senses from the block it arrives on; the mood's `mood_index` counts in the
//!   module's own `moods` list (A1: its place + 1, 0 = not in it – deck-space `context::mood_index`).
//!
//! The module runtime is a seam: the platform hands the engine a `ModuleSource` (the native deck: wasmtime behind
//! deck-module-host's `Backend`) and the engine drives the instances it opens through `ModuleInstance` – implemented for
//! deck-module-host's `Instance<B>`, so the host logic stays in that crate. Nothing here allocates per block (a show's
//! restart aside: its job is boxed when a trap happened); the module's own code may not allocate either.
use super::control::Context;
use super::{Block, Buf, Handles, Param, QUANTUM};
use anyhow::{anyhow, Result};
use deck_core::tempo::{BeatReader, TempoMap};
use deck_module_host::{Backend, Ev, InBind, Layout, OutBind};
use deck_space::context::mood_index;
use deck_space::graph::{Graph, Kind, Node, MAX_CHANNELS};
use deck_space::modreg::{self, ModuleDef, ModuleKind};
use std::any::Any;
use std::sync::Arc;

/// What the engine drives of one running module (deck-module-host's `Instance`, whatever its backend).
pub trait ModuleInstance: Send {
    /// set once a trap or an overrun silenced the instance (it then renders zeros)
    fn silenced(&self) -> bool;
    fn set_character(&mut self, id: &str, persona: &str, mood: &str);
    /// the mood's place in the module's own `moods` list + 1 (A1, #409; 0 = not in it)
    fn set_mood_index(&mut self, _i: u32) {}
    /// the being's senses, 0 … 1 (#409)
    fn set_senses(&mut self, _energy: f32, _touch: f32, _strain: f32) {}
    fn set_vars(&mut self, vars: &[(String, f32)]);
    fn set_tempo(&mut self, map: &TempoMap);
    fn set_param(&mut self, index: usize, value: f32);
    fn set_control_in(&mut self, port: usize, value: f32);
    /// audio input `port` for the next call: channel slices of the cable (empty = nothing linked)
    fn set_audio_in(&mut self, port: usize, chans: &[&[f32]]);
    /// the channels the engine takes out of audio output `port` (an `any` output follows it)
    fn set_out_channels(&mut self, _port: usize, _ch: usize) {}
    /// one call of `frames` (≤ `QUANTUM`) frames from `frame` – `dm_process` or `dm_control`, the module's domain;
    /// false when it did not run
    fn process(&mut self, frame: f64, frames: u32) -> bool;
    /// channel `c` of audio output `port` of the last call into `dst` (silent where the output has no such channel)
    fn read_audio_out(&mut self, port: usize, c: usize, dst: &mut [f32]);
    /// control output `i` of the last call and whether it was written
    fn control_out(&self, _i: usize) -> (f32, bool) {
        (0.0, false)
    }
    /// event input `port`'s records for the next call (sorted by the host); how many it took
    fn set_events_in(&mut self, _port: usize, _events: &[Ev]) -> usize {
        0
    }
    /// `n` of the next call's events came late (clamped to frame 0)
    fn add_late(&mut self, _n: u32) {}
    /// event output `port`'s records of the last call
    fn events_out(&self, _port: usize) -> &[Ev] {
        &[]
    }
    /// the next call swaps a rebuilt instance in (a show's restart)
    fn swapping(&self) -> bool {
        false
    }
    /// a show's restart, once its pause is over: the job for the platform's loader thread
    fn take_restart(&mut self) -> Option<Box<dyn Restart>> {
        None
    }
    /// what a restart job returned
    fn offer(&mut self, _done: Box<dyn Any + Send>) {}
}

/// A restart's rebuild, run off the audio thread by the platform (`Restarts::serve`).
pub trait Restart: Send {
    fn run(self: Box<Self>) -> Box<dyn Any + Send>;
}

impl<B: Backend + Send + 'static> Restart for deck_module_host::Rebuild<B> {
    fn run(self: Box<Self>) -> Box<dyn Any + Send> {
        Box::new((*self).run())
    }
}

impl<B: Backend + Send + 'static> ModuleInstance for deck_module_host::Instance<B> {
    fn silenced(&self) -> bool {
        deck_module_host::Instance::silenced(self).is_some()
    }
    fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        deck_module_host::Instance::set_character(self, id, persona, mood)
    }
    fn set_mood_index(&mut self, i: u32) {
        deck_module_host::Instance::set_mood_index(self, i)
    }
    fn set_senses(&mut self, energy: f32, touch: f32, strain: f32) {
        deck_module_host::Instance::set_senses(self, energy, touch, strain)
    }
    fn set_vars(&mut self, vars: &[(String, f32)]) {
        deck_module_host::Instance::set_vars(self, vars)
    }
    fn set_tempo(&mut self, map: &TempoMap) {
        deck_module_host::Instance::set_tempo(self, map)
    }
    fn set_param(&mut self, index: usize, value: f32) {
        deck_module_host::Instance::set_param(self, index, value)
    }
    fn set_control_in(&mut self, port: usize, value: f32) {
        deck_module_host::Instance::set_control_in(self, port, value)
    }
    fn set_audio_in(&mut self, port: usize, chans: &[&[f32]]) {
        self.set_audio_chans(port, chans)
    }
    fn set_out_channels(&mut self, port: usize, ch: usize) {
        deck_module_host::Instance::set_out_channels(self, port, ch)
    }
    fn process(&mut self, frame: f64, frames: u32) -> bool {
        deck_module_host::Instance::process(self, frame, frames)
    }
    fn read_audio_out(&mut self, port: usize, c: usize, dst: &mut [f32]) {
        self.read_audio_chan(port, c, dst)
    }
    fn control_out(&self, i: usize) -> (f32, bool) {
        deck_module_host::Instance::control_out(self, i)
    }
    fn set_events_in(&mut self, port: usize, events: &[Ev]) -> usize {
        deck_module_host::Instance::set_events_in(self, port, events)
    }
    fn add_late(&mut self, n: u32) {
        deck_module_host::Instance::add_late(self, n)
    }
    fn events_out(&self, port: usize) -> &[Ev] {
        deck_module_host::Instance::events_out(self, port)
    }
    fn swapping(&self) -> bool {
        deck_module_host::Instance::swapping(self)
    }
    fn take_restart(&mut self) -> Option<Box<dyn Restart>> {
        self.take_rebuild().map(|r| Box::new(r) as Box<dyn Restart>)
    }
    fn offer(&mut self, done: Box<dyn Any + Send>) {
        if let Ok(fresh) = done.downcast::<deck_module_host::Fresh<B>>() {
            deck_module_host::Instance::offer(self, *fresh);
        }
    }
}

/// Registers a loaded module as the node kind `module:<name>` (deck-space modreg): its graph ports, their channels, the
/// parameters' defaults, its domain and path targets – as the host's `layout` binds them. The one mapping from a module
/// to the graph; a platform's library calls it when it loads a module.
pub fn register(desc: &deck_catalog::Descriptor, layout: &Layout) -> ModuleKind {
    // deck-space's ModuleDef::of numbers the ports by the catalog's rule – the one the host's layout binds by
    let def = ModuleDef::of(desc);
    debug_assert_eq!(def.inputs.iter().filter(|p| p.ty != deck_space::graph::PortType::Event).count(), layout.in_ports.len(), "the graph and the host number a module's ports alike");
    modreg::register(&desc.name, &def)
}

/// A module opened for a node: the instance and how the graph's ports map into it (the host's `Layout`).
pub struct Opened {
    pub instance: Box<dyn ModuleInstance>,
    pub layout: Layout,
    /// how many parameters the module has
    pub params: usize,
}

/// Where the platform's loaded modules are (the native deck: its library of `.wasm` files, compiled once).
pub trait ModuleSource: Send + Sync {
    /// the module behind `kind`, instantiated for `rate` Hz with `seed`; Err = not loaded / failed to instantiate
    fn open(&self, kind: ModuleKind, rate: f64, seed: u64) -> Result<Opened>;
    /// an instance was silenced (the platform's HUD and tests can look); called once per instance, from the render
    fn silenced(&self) {}
}

/// how many restart jobs may wait per node (one is out at a time)
const RESTART_RING: usize = 2;

/// The platform's side of one module node's restarts (a show, ABI-V2 §6): the engine hands a trapped instance's rebuild
/// out here and takes the result back at its next block. `serve` runs what waits – on the platform's loader thread,
/// never the audio thread (instantiating allocates). `Handles::restarts` has one per module node.
pub struct Restarts {
    /// the node's id (for the log)
    pub node: String,
    jobs: rtrb::Consumer<Box<dyn Restart>>,
    done: rtrb::Producer<Box<dyn Any + Send>>,
}

impl Restarts {
    /// Runs the waiting rebuilds and hands their results back; how many ran.
    pub fn serve(&mut self) -> usize {
        let mut ran = 0;
        while let Ok(job) = self.jobs.pop() {
            // one job is out at a time, so the lane back has room
            let _ = self.done.push(job.run());
            ran += 1;
        }
        ran
    }

    /// the engine that owned the node is gone (the platform's loader may stop serving the lane)
    pub fn abandoned(&self) -> bool {
        self.jobs.is_abandoned()
    }
}

/// A module node opened at build – before its block or stepper exists, so the event ports can be wired across the whole
/// graph first.
pub(crate) struct Prepared {
    opened: Opened,
    /// (graph output, value) a bound path output holds until the module writes it (deck-space's `Graph::holds`)
    holds: Vec<(usize, f32)>,
    #[cfg(feature = "events")]
    pub(crate) ports: Option<super::evhost::Ports>,
}

impl Prepared {
    #[cfg(feature = "events")]
    pub(crate) fn layout(&self) -> &Layout {
        &self.opened.layout
    }
}

/// a node's `seed` param: 1 unless the node sets it (owner 2026-11-03: "starting with the same is fine")
pub(crate) fn seed_of(node: &Node) -> u64 {
    match node.params.get("seed") {
        Some(toml::Value::Integer(i)) => *i as u64,
        Some(toml::Value::Float(f)) => *f as u64,
        _ => 1,
    }
}

/// Opens every module node of `graph` (audio and control domain) and wires their event ports; None for other nodes.
pub(crate) fn prepare(graph: &Graph, rate: f32, source: Option<&Arc<dyn ModuleSource>>, h: &mut Handles, seams: &super::Seams) -> Result<Vec<Option<Prepared>>> {
    #[cfg(not(feature = "events"))]
    let _ = (&h, seams);
    let mut out: Vec<Option<Prepared>> = (0..graph.nodes.len()).map(|_| None).collect();
    for (i, node) in graph.nodes.iter().enumerate() {
        let Kind::Module(kind) = node.kind else { continue };
        let source = source.ok_or_else(|| anyhow!("node '{}': this engine has no module runtime", node.id))?;
        let opened = source.open(kind, f64::from(rate), seed_of(node)).map_err(|e| anyhow!("node '{}': {e:#}", node.id))?;
        let holds = graph.holds.iter().filter(|((n, _), _)| *n == i).map(|&((_, o), v)| (o, v)).collect();
        out[i] = Some(Prepared {
            opened,
            holds,
            #[cfg(feature = "events")]
            ports: None,
        });
    }
    #[cfg(feature = "events")]
    super::evhost::wire(graph, &mut out, &mut h.events, seams.tuning)?;
    Ok(out)
}

/// What both kinds of module node share: the instance, the clock, the control outputs' Params, the restart lane, the
/// event ports.
struct Core {
    inst: Box<dyn ModuleInstance>,
    source: Arc<dyn ModuleSource>,
    beats: BeatReader,
    /// without a tempo clock: 120 bpm from frame 0, counted here
    own: TempoMap,
    frame: f64,
    /// per control output of the module: its Param (the control half copies it into the output's slot)
    controls: Vec<Param>,
    reported: bool,
    jobs: rtrb::Producer<Box<dyn Restart>>,
    done: rtrb::Consumer<Box<dyn Any + Send>>,
    #[cfg(feature = "events")]
    ports: Option<super::evhost::Ports>,
    /// the character the node plays for (its `character` param; the live context does not move it)
    id: String,
    /// the module's own moods (A1): a live mood's `mood_index` counts in them
    moods: &'static [&'static str],
}

impl Core {
    /// The instance with its context set (character, variables), a Param per control output (holding `holds`' value
    /// until written – registered as the control half's feed), and a restart lane in `h`.
    fn new(index: usize, node: &Node, rate: f32, p: Prepared, beats: BeatReader, source: &Arc<dyn ModuleSource>, h: &mut Handles) -> (Core, Layout, usize) {
        #[cfg(feature = "events")]
        let ports = p.ports;
        let Opened { instance: mut inst, layout, params } = p.opened;
        let holds = p.holds;
        let id = node.str("character").unwrap_or("").to_string();
        let moods = match node.kind {
            Kind::Module(k) => k.spec().moods,
            _ => &[],
        };
        // the node's params until the deck's first live context (#409)
        let mood = node.str("mood").unwrap_or("");
        inst.set_character(&id, node.str("persona").unwrap_or(""), mood);
        inst.set_mood_index(mood_index(moods, mood));
        if let Some(toml::Value::Table(t)) = node.params.get("vars") {
            let vars: Vec<(String, f32)> = t
                .iter()
                .filter_map(|(k, v)| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)).map(|f| (k.clone(), f)))
                .collect();
            inst.set_vars(&vars);
        }
        let mut controls = Vec::new();
        for (port, bind) in layout.outputs.iter().enumerate() {
            if let OutBind::Control(_) = bind {
                let hold = holds.iter().find(|(o, _)| *o == port).map_or(0.0, |&(_, v)| v);
                let param = Param::new(hold);
                h.feeds.push((index, port, param.clone()));
                controls.push(param);
            }
        }
        let (jobs, rx) = rtrb::RingBuffer::new(RESTART_RING);
        let (tx, done) = rtrb::RingBuffer::new(RESTART_RING);
        h.restarts.push(Restarts { node: node.id.clone(), jobs: rx, done: tx });
        let core = Core {
            inst,
            source: source.clone(),
            beats,
            own: TempoMap::new(120.0, 4, f64::from(rate)),
            frame: 0.0,
            controls,
            reported: false,
            jobs,
            done,
            #[cfg(feature = "events")]
            ports,
            id,
            moods,
        };
        (core, layout, params)
    }

    /// The deck's live context (#409) into the module's header for its next call: the mood and persona (the character
    /// stays the node's), the mood's index in the module's own list (A1), the senses. No allocation (bytes into the
    /// header, a compare per listed mood).
    fn context(&mut self, c: &Context) {
        self.inst.set_character(&self.id, c.persona(), c.mood());
        self.inst.set_mood_index(mood_index(self.moods, c.mood()));
        self.inst.set_senses(c.energy, c.touch, c.strain);
    }

    /// the block's tempo map and first frame (the deck's clock, or our own count without one)
    fn clock(&mut self, n: usize) -> (TempoMap, f64) {
        let (map, start) = match self.beats.block() {
            Some((m, _, start)) => (m, start),
            None => (self.own, self.frame),
        };
        self.frame = start + n as f64;
        (map, start)
    }

    /// Before a call over [start, start + n): a rebuild that came back is offered, the tempo set, the due events in.
    fn begin(&mut self, map: &TempoMap, start: f64, n: usize) {
        while let Ok(done) = self.done.pop() {
            self.inst.offer(done);
        }
        self.inst.set_tempo(map);
        #[cfg(feature = "events")]
        if let Some(p) = &mut self.ports {
            p.deliver(self.inst.as_mut(), map, start, n);
        }
        #[cfg(not(feature = "events"))]
        let _ = (start, n);
    }

    /// After a call: the written control outputs into their Params, the events out, a restart's job to the platform,
    /// a silencing reported once. False when the instance is silent for good.
    fn end(&mut self, ran: bool, map: &TempoMap, start: f64, n: usize) -> bool {
        if ran {
            for (i, p) in self.controls.iter().enumerate() {
                let (v, written) = self.inst.control_out(i);
                if written {
                    p.set(v);
                }
            }
        }
        #[cfg(feature = "events")]
        if let Some(p) = &mut self.ports {
            p.collect(self.inst.as_ref(), ran, map, start, n);
        }
        #[cfg(not(feature = "events"))]
        let _ = (map, start, n);
        if let Some(job) = self.inst.take_restart() {
            // a full lane cannot happen (one job is out at a time); were it full, the job (and the dead instance) drops
            let _ = self.jobs.push(job);
        }
        if self.inst.silenced() {
            if !self.reported {
                self.reported = true;
                self.source.silenced();
            }
            return false;
        }
        true
    }
}

/// An `audio` module: a block of the audio half.
pub(super) struct ModuleBlock {
    core: Core,
    /// per graph input: where it goes in the module, and its param
    inputs: Vec<(InBind, Param)>,
    /// the parameter values last handed over (bits, so NaN compares)
    sent: Vec<u32>,
    outputs: Vec<OutBind>,
    /// the output channel counts told to the module (at the first block)
    told: bool,
}

impl ModuleBlock {
    pub(super) fn new(index: usize, node: &Node, rate: f32, params: &[Param], beats: BeatReader, source: &Arc<dyn ModuleSource>, h: &mut Handles) -> Result<ModuleBlock> {
        let p = h.prepared.get_mut(index).and_then(Option::take).ok_or_else(|| anyhow!("node '{}': its module was not opened", node.id))?;
        let (mut core, layout, count) = Core::new(index, node, rate, p, beats, source, h);
        let inputs: Vec<(InBind, Param)> = layout.inputs.iter().copied().zip(params.iter().cloned()).collect();
        let mut sent = vec![f32::NAN.to_bits(); count];
        for (bind, p) in &inputs {
            if let InBind::Param(i) = bind {
                core.inst.set_param(*i, p.get());
                sent[*i] = p.get().to_bits();
            }
        }
        Ok(ModuleBlock { core, inputs, sent, outputs: layout.outputs, told: false })
    }

    /// one call over the frames [off, off + m) of the block (m ≤ QUANTUM)
    fn call(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], map: &TempoMap, start: f64, off: usize, m: usize) {
        let core = &mut self.core;
        if !core.inst.silenced() {
            for (k, (bind, param)) in self.inputs.iter().enumerate() {
                match *bind {
                    InBind::Audio(port) => match ins.get(k).copied().flatten() {
                        Some(b) => {
                            let mut chans: [&[f32]; MAX_CHANNELS] = [&[]; MAX_CHANNELS];
                            let ch = b.channels().min(MAX_CHANNELS);
                            for (c, s) in chans.iter_mut().enumerate().take(ch) {
                                *s = &b.ch(c)[off..off + m];
                            }
                            core.inst.set_audio_in(port, &chans[..ch]);
                        }
                        None => core.inst.set_audio_in(port, &[]),
                    },
                    InBind::Control(port) => core.inst.set_control_in(port, param.get()),
                    InBind::Param(i) => {
                        let v = param.get();
                        if v.to_bits() != self.sent[i] {
                            self.sent[i] = v.to_bits();
                            core.inst.set_param(i, v);
                        }
                    }
                }
            }
        }
        core.begin(map, start, m);
        let ran = !core.inst.silenced() && core.inst.process(start, m as u32);
        let alive = core.end(ran, map, start, m);
        for (o, bind) in outs.iter_mut().zip(&self.outputs) {
            let OutBind::Audio(port) = *bind else { continue };
            for c in 0..o.channels() {
                let dst = &mut o.ch_mut(c)[off..off + m];
                if alive {
                    core.inst.read_audio_out(port, c, dst);
                } else {
                    dst.fill(0.0);
                }
            }
        }
    }
}

impl Block for ModuleBlock {
    fn context(&mut self, c: &Context) {
        self.core.context(c);
    }

    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        if !self.told {
            self.told = true;
            for (o, bind) in outs.iter().zip(&self.outputs) {
                if let OutBind::Audio(port) = *bind {
                    self.core.inst.set_out_channels(port, o.channels());
                }
            }
        }
        let (map, start) = self.core.clock(n);
        // a v2 call is at most a quantum; a longer block (the tests drive blocks by hand) is cut into quanta
        let mut off = 0;
        while off < n {
            let m = (n - off).min(QUANTUM);
            self.call(ins, outs, &map, start + off as f64, off, m);
            off += m;
        }
    }
}

/// A `control` module (ABI-V2 §5.2): a stepper of the control half – `dm_control` once per block, after the control
/// levels; its inputs are the control half's values of its graph inputs, its outputs act at the next block.
pub(super) struct ControlModule {
    core: Core,
    inputs: Vec<InBind>,
    sent: Vec<u32>,
}

impl ControlModule {
    pub(super) fn new(index: usize, node: &Node, rate: f32, p: Prepared, beats: BeatReader, source: &Arc<dyn ModuleSource>, h: &mut Handles) -> ControlModule {
        let (core, layout, count) = Core::new(index, node, rate, p, beats, source, h);
        ControlModule { core, inputs: layout.inputs, sent: vec![f32::NAN.to_bits(); count] }
    }
}

impl super::control::Stepper for ControlModule {
    fn context(&mut self, c: &Context) {
        self.core.context(c);
    }

    fn step(&mut self, ins: &[f32], _frame: u64, n: usize) {
        let (map, start) = self.core.clock(n);
        let core = &mut self.core;
        if !core.inst.silenced() {
            for (bind, &v) in self.inputs.iter().zip(ins) {
                match *bind {
                    InBind::Control(port) => core.inst.set_control_in(port, v),
                    InBind::Param(i) => {
                        if v.to_bits() != self.sent[i] {
                            self.sent[i] = v.to_bits();
                            core.inst.set_param(i, v);
                        }
                    }
                    // a control module has no audio port (the SDK and the host refuse one)
                    InBind::Audio(_) => {}
                }
            }
        }
        let mut off = 0;
        while off < n {
            let m = (n - off).min(QUANTUM);
            let at = start + off as f64;
            core.begin(&map, at, m);
            let ran = !core.inst.silenced() && core.inst.process(at, m as u32);
            core.end(ran, &map, at, m);
            off += m;
        }
    }
}

#[cfg(test)]
#[path = "module_tests.rs"]
mod tests;
