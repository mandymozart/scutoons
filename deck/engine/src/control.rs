//! The control domain – ONE control half (#319, research/ENGINE-REFACTOR-PLAN.md P5, §2.8): the engine steps it once per
//! 128-frame quantum inside the render (`Control`), and the picture reads what it computed from a snapshot ring by the
//! heard frame (`Controls`, the frame thread's face). Before #319 the native deck stepped it once per video frame – the
//! screen's refresh drove the sound – and the browser ran deck-core's `BatchGraph` (retired with #324); its API (`send` with an amount,
//! `set_base`, `input`, `nodes`, the helpers `smooth` / `mul` / `add`) is this one's now.
//!
//! Compiled at load (#183, the batch model): one value slot per control output, every control input as a column
//! (`cols::Col`: the linked slot × scale + offset, or a constant), the computed nodes (lfo, smooth, mul, add)
//! in dependency levels, one batch per level and operation – struct-of-arrays, one loop each. A step: the FFT analyzers,
//! the tempo node (the clock at the block's first frame), the levels, then every control input of an audio node into
//! its Param (the audio blocks glide each across the block – smooth.rs: per-sample interpolation, so a value that moves
//! every block never zippers). Allocates only at build; a step, a `set_base`, a `send`, a shape change never do.
//!
//! What the frame thread may change (a MIDI CC, a 3D button, the being's waveform, a sent value) goes to the engine as a
//! `Command` through a lock-free ring; the face keeps its own copy of the input table, so it reads every input of the
//! heard snapshot without asking the audio thread. The live context (#409: the deck's mood, persona and the being's
//! senses, deck-space context.rs) is one such command: applied at the next block's start, the control half hands it to
//! its module steppers at once and the engine to its audio module blocks before they run (lib.rs `quantum`).
use crate::cols::{levels, Col};
use crate::fft::Analyzer;
use crate::Param;
use deck_core::math::{fract_fast, sin_turns};
use deck_core::tempo::{self, BeatReader};
use deck_space::graph::{Graph, Kind, PortType};
use deck_space::params::default_for;
use std::sync::atomic::{fence, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// an LFO's shapes by name (its `shape` param), in the order `lfo_shape` counts them (deck-space's params.rs)
pub use deck_space::params::LFO_SHAPES;
/// the live context the module nodes play in (#409)
pub use deck_space::context::Context;

/// how many blocks the snapshot ring holds (64 × 128 frames ≈ 170 ms at 48 kHz: more than the lead + output latency)
pub const DEPTH: usize = 64;
/// how many commands may wait for the next block (a frame's knob drags and sends; more wait in the face)
const COMMANDS: usize = 1024;

/// Where a control input's value comes from (fixed at load but for its base: `set_base` turns it).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Src {
    /// a link: `values[slot] * scale + base` (the base is the link's offset as built)
    Link(u32, f32, f32),
    /// the node's param of the same name (or a base set from outside)
    Param(f32),
    /// neither: the reader's default
    Default,
}

/// One control input: its source and the last value sent onto it (`send`: amount 0 = none).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Input {
    src: Src,
    value: f32,
    amount: f32,
}

impl Input {
    /// the column form (slot, scale, offset); `default` stands for `Src::Default`. A send blends affinely:
    /// `own · (1 − amount) + value · amount` – one column change, nothing per block.
    fn col(&self, default: f32) -> (u32, f32, f32) {
        let (slot, scale, base) = match self.src {
            Src::Link(slot, scale, base) => (slot, scale, base),
            Src::Param(v) => (0, 0.0, v),
            Src::Default => (0, 0.0, default),
        };
        if self.amount == 0.0 {
            return (slot, scale, base);
        }
        let a = self.amount;
        (slot, scale * (1.0 - a), base * (1.0 - a) + self.value * a)
    }

    fn read(&self, v: &[f32], default: f32) -> f32 {
        let (slot, scale, offset) = self.col(default);
        v[slot as usize] * scale + offset
    }
}

/// The input table both halves keep: per node its kind, its first value slot, its control inputs; the LFOs' shapes.
#[derive(Clone)]
struct Table {
    ids: Vec<String>,
    kinds: Vec<Kind>,
    /// node n's outputs from slot `base[n]` (slot 0 stays 0: the constant source)
    base: Vec<u32>,
    slots: usize,
    inputs: Vec<Vec<Input>>,
    /// per node: an LFO's shape (an index into LFO_SHAPES)
    shapes: Vec<Option<usize>>,
}

impl Table {
    fn new(graph: &Graph) -> Table {
        let mut base = Vec::with_capacity(graph.nodes.len());
        let mut slots = 1u32;
        for n in &graph.nodes {
            base.push(slots);
            slots += n.kind.outputs().len() as u32;
        }
        let inputs = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                n.kind
                    .inputs()
                    .iter()
                    .enumerate()
                    .map(|(k, port)| {
                        let src = match graph.link_into(i, k) {
                            Some(l) if l.ty == PortType::Control => Src::Link(base[l.from.0] + l.from.1 as u32, l.scale, l.offset),
                            _ if n.params.contains_key(port.name) => Src::Param(n.f32(port.name, 0.0)),
                            _ => Src::Default,
                        };
                        Input { src, value: 0.0, amount: 0.0 }
                    })
                    .collect()
            })
            .collect();
        let shapes = graph
            .nodes
            .iter()
            .map(|n| (n.kind == Kind::Lfo).then(|| Shape::parse(n.str("shape").unwrap_or("sine")) as usize))
            .collect();
        Table {
            ids: graph.nodes.iter().map(|n| n.id.clone()).collect(),
            kinds: graph.nodes.iter().map(|n| n.kind).collect(),
            base,
            slots: slots as usize,
            inputs,
            shapes,
        }
    }

    fn get(&self, node: usize, port: usize) -> Option<&Input> {
        self.inputs.get(node)?.get(port)
    }

    /// the input's base (#188): a linked input's link offset, an unlinked one's value
    fn set_base(&mut self, node: usize, port: usize, base: f32) -> bool {
        let Some(input) = self.inputs.get_mut(node).and_then(|i| i.get_mut(port)) else { return false };
        input.src = match input.src {
            Src::Link(slot, scale, _) => Src::Link(slot, scale, base),
            _ => Src::Param(base),
        };
        true
    }

    fn send(&mut self, node: usize, port: usize, value: f32, amount: f32) -> bool {
        let Some(input) = self.inputs.get_mut(node).and_then(|i| i.get_mut(port)) else { return false };
        (input.value, input.amount) = (value, amount.clamp(0.0, 1.0));
        true
    }

    fn base_of(&self, node: usize, port: usize) -> Option<f32> {
        match self.get(node, port)?.src {
            Src::Link(_, _, base) => Some(base),
            _ => None,
        }
    }

    fn input(&self, v: &[f32], node: usize, port: &str, default: f32) -> f32 {
        let Some(k) = self.kinds.get(node).and_then(|k| k.input(port)) else { return default };
        self.inputs[node][k].read(v, default)
    }

    /// by port index, an unset input at the catalog's default
    fn input_at(&self, v: &[f32], node: usize, port: usize) -> Option<f32> {
        let input = self.get(node, port)?;
        Some(input.read(v, default_for(self.kinds[node], self.kinds[node].inputs()[port].name)))
    }

    fn outputs<'a>(&self, v: &'a [f32], node: usize) -> &'a [f32] {
        let b = self.base[node] as usize;
        &v[b..b + self.kinds[node].outputs().len()]
    }

    fn nodes(&self) -> impl Iterator<Item = (&str, &'static str, Vec<&'static str>)> + '_ {
        self.ids.iter().zip(&self.kinds).map(|(id, k)| (id.as_str(), k.name(), k.inputs().iter().map(|p| p.name).collect()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Sine,
    Saw,
    Square,
}

impl Shape {
    fn parse(s: &str) -> Shape {
        match s {
            "saw" => Shape::Saw,
            "square" => Shape::Square,
            _ => Shape::Sine,
        }
    }

    fn of(i: usize) -> Shape {
        [Shape::Sine, Shape::Saw, Shape::Square][i.min(2)]
    }
}

/// What a batch computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Lfo(Shape),
    Smooth,
    Mul,
    Add,
}

impl Op {
    fn of(kind: Kind, shape: Option<usize>) -> Option<Op> {
        Some(match kind {
            Kind::Lfo => Op::Lfo(Shape::of(shape.unwrap_or(0))),
            Kind::Smooth => Op::Smooth,
            Kind::Mul => Op::Mul,
            Kind::Add => Op::Add,
            _ => return None,
        })
    }
}

/// One level's nodes of one operation: two input columns (lfo rate · depth, smooth in · time, mul / add a · b), a state
/// per node (the LFO's phase, the smoother's value), where each writes, and which node each is.
struct Batch {
    op: Op,
    a: Col,
    b: Col,
    state: Vec<f32>,
    out: Vec<u32>,
    node: Vec<u32>,
}

impl Batch {
    fn new(op: Op, room: usize) -> Batch {
        let (mut a, mut b) = (Col::default(), Col::default());
        a.reserve(room);
        b.reserve(room);
        Batch { op, a, b, state: Vec::with_capacity(room), out: Vec::with_capacity(room), node: Vec::with_capacity(room) }
    }

    /// One loop over the batch – no dispatch, no branch on an input's kind (vectorisable).
    #[inline(always)]
    fn step(&mut self, v: &mut [f32], dt: f32) {
        let (a, b) = (&self.a, &self.b);
        match self.op {
            Op::Lfo(shape) => {
                for i in 0..self.out.len() {
                    let ph = fract_fast(self.state[i] + a.get(v, i) * dt);
                    self.state[i] = ph;
                    let wave = match shape {
                        Shape::Sine => sin_turns(ph),
                        Shape::Saw => ph * 2.0 - 1.0,
                        Shape::Square => {
                            if ph < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                    };
                    v[self.out[i] as usize] = (0.5 + 0.5 * wave) * b.get(v, i);
                }
            }
            Op::Smooth => {
                for i in 0..self.out.len() {
                    let t = b.get(v, i);
                    let k = if t <= 0.0 { 1.0 } else { 1.0 - libm::expf(-dt / t) };
                    self.state[i] += (a.get(v, i) - self.state[i]) * k;
                    v[self.out[i] as usize] = self.state[i];
                }
            }
            Op::Mul => {
                for i in 0..self.out.len() {
                    v[self.out[i] as usize] = a.get(v, i) * b.get(v, i);
                }
            }
            Op::Add => {
                for i in 0..self.out.len() {
                    v[self.out[i] as usize] = a.get(v, i) + b.get(v, i);
                }
            }
        }
    }
}

/// What the frame thread asks of the engine's control half; applied at the next block's start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Base { node: u32, port: u32, value: f32 },
    Send { node: u32, port: u32, value: f32, amount: f32 },
    Shape { node: u32, shape: u32 },
    /// the live context for every module node (#409; plain bytes – no allocation crosses)
    Context(Context),
}

/// The engine's side of the link to the frame thread: the commands it applies and the ring it writes.
pub(crate) struct Feed {
    commands: rtrb::Consumer<Command>,
    snapshots: Arc<Snapshots>,
}

/// What the frame thread's `Controls` takes from a built engine (`Handles::control`).
pub struct Link {
    commands: rtrb::Producer<Command>,
    snapshots: Arc<Snapshots>,
    frames: Arc<AtomicU64>,
}

/// The engine's link pair: its feed and the face's link.
pub(crate) fn link(slots: usize, frames: Arc<AtomicU64>) -> (Feed, Link) {
    let (tx, rx) = rtrb::RingBuffer::new(COMMANDS);
    let snapshots = Arc::new(Snapshots::new(slots));
    (Feed { commands: rx, snapshots: snapshots.clone() }, Link { commands: tx, snapshots, frames })
}

/// The picture's view of the control half (plan §3.3): every block's values, stamped with the block's first frame, in a
/// preallocated ring `DEPTH` blocks deep. One writer (the engine), any reader; a per-entry seqlock (the frame stamp: a
/// new entry always has a new frame) – never a lock, never a wait, never an allocation.
pub struct Snapshots {
    slots: usize,
    frames: Box<[AtomicU64]>,
    values: Box<[AtomicU32]>,
    head: AtomicU64,
}

/// an entry being written (or never written)
const EMPTY: u64 = u64::MAX;

impl Snapshots {
    pub fn new(slots: usize) -> Snapshots {
        Snapshots {
            slots,
            frames: (0..DEPTH).map(|_| AtomicU64::new(EMPTY)).collect(),
            values: (0..DEPTH * slots).map(|_| AtomicU32::new(0)).collect(),
            head: AtomicU64::new(0),
        }
    }

    /// the values of the block starting at `frame` (the engine, once per block)
    pub fn write(&self, frame: u64, values: &[f32]) {
        let n = self.head.load(Ordering::Relaxed);
        let e = (n % DEPTH as u64) as usize;
        self.frames[e].store(EMPTY, Ordering::Relaxed);
        fence(Ordering::Release);
        for (d, x) in self.values[e * self.slots..(e + 1) * self.slots].iter().zip(values) {
            d.store(x.to_bits(), Ordering::Relaxed);
        }
        self.frames[e].store(frame, Ordering::Release);
        self.head.store(n + 1, Ordering::Release);
    }

    /// The values of the newest block that starts at or before `frame` into `out` (the oldest kept one if `frame` is
    /// older still). Returns that block's first frame; None = nothing written yet (`out` keeps what it had).
    pub fn read(&self, frame: u64, out: &mut [f32]) -> Option<u64> {
        for _ in 0..4 {
            let n = self.head.load(Ordering::Acquire);
            let mut pick = None;
            for back in 0..(n as usize).min(DEPTH) {
                let e = ((n - 1 - back as u64) % DEPTH as u64) as usize;
                let f = self.frames[e].load(Ordering::Acquire);
                if f == EMPTY {
                    continue;
                }
                pick = Some(e);
                if f <= frame {
                    break;
                }
            }
            let e = pick?;
            let f1 = self.frames[e].load(Ordering::Acquire);
            if f1 == EMPTY {
                continue;
            }
            for (o, s) in out.iter_mut().zip(&self.values[e * self.slots..(e + 1) * self.slots]) {
                *o = f32::from_bits(s.load(Ordering::Relaxed));
            }
            fence(Ordering::Acquire);
            if self.frames[e].load(Ordering::Relaxed) == f1 {
                return Some(f1);
            }
        }
        None
    }
}

/// A node of the control half computed outside the batches – a `domain: control` module (#342, module.rs): stepped
/// once per block after the levels and the audio nodes' Params; `ins` = its graph inputs' values this block. What it
/// writes reaches the slots through a feed at the next block (one block late, like every module output).
pub(crate) trait Stepper: Send {
    fn step(&mut self, ins: &[f32], frame: u64, n: usize);
    /// the live context changed (#409): from this block on
    fn context(&mut self, _: &Context) {}
}

struct Stepped {
    node: usize,
    ins: Vec<f32>,
    stepper: Box<dyn Stepper>,
}

/// The control half itself: built with the engine and stepped by it once per block (or by a face without an engine).
pub struct Control {
    table: Table,
    /// this block's value of every control output
    values: Vec<f32>,
    /// the computed nodes, level by level (an LFO level has a batch per shape, room for all its LFOs in each)
    levels: Vec<Vec<Batch>>,
    /// per node: where it sits in the batches (level, batch, index)
    at: Vec<Option<(usize, usize, usize)>>,
    tempo_node: Option<usize>,
    analyzers: Vec<(usize, Analyzer)>,
    /// every control input of an audio node: a column, its Param, whose input it is, the default for an unset one
    param_in: Col,
    params: Vec<Param>,
    param_at: Vec<(usize, usize)>,
    param_default: Vec<f32>,
    /// the engine's: the block's clock and the frame thread's link (None for a face's own)
    beats: Option<BeatReader>,
    feed: Option<Feed>,
    /// the reference cadence of the P5 golden diff: step only every `period` frames (the frame thread's 60 Hz before
    /// #319), with dt one period – (the next frame due, the period in frames)
    cadence: Option<(f64, f64)>,
    /// the clock this block (the tempo node's outputs and quantised cuts read it)
    pub tempo: tempo::Now,
    /// values written outside the step – a module's control outputs (#342): (slot, the Param), copied into the slots at
    /// the start of each step, so what a module wrote in block k is read in block k + 1
    feeds: Vec<(u32, Param)>,
    /// the `domain: control` modules, in scene order
    steppers: Vec<Stepped>,
    /// the live context last applied (#409), and whether the audio half has yet to get it (`take_context`)
    context: Context,
    fresh: bool,
}

impl Control {
    /// The control half of `graph`; `params` = every control input of the engine's audio nodes (graph node, input port,
    /// the Param its block reads), `taps` = the fft nodes' sample rings, `rate` the engine's (for the analyzers).
    pub(crate) fn new(graph: &Graph, params: &[(usize, usize, Param)], taps: Vec<(usize, rtrb::Consumer<f32>)>, rate: f32) -> Control {
        let table = Table::new(graph);
        let n = graph.nodes.len();
        let mut c = Control {
            values: vec![0.0; table.slots],
            levels: Vec::new(),
            at: vec![None; n],
            tempo_node: graph.first(Kind::Tempo),
            analyzers: taps.into_iter().map(|(node, ring)| (node, Analyzer::new(ring, rate, graph.nodes[node].f32("release", 0.25)))).collect(),
            param_in: Col::default(),
            params: Vec::with_capacity(params.len()),
            param_at: Vec::with_capacity(params.len()),
            param_default: Vec::with_capacity(params.len()),
            beats: None,
            feed: None,
            cadence: None,
            tempo: tempo::Now::default(),
            feeds: Vec::new(),
            steppers: Vec::new(),
            context: Context::default(),
            fresh: false,
            table,
        };
        // the computed nodes in levels: a node reading another computed node sits a level above it
        let op_of = |i: usize| Op::of(graph.nodes[i].kind, c.table.shapes[i]);
        let deps: Vec<Vec<usize>> = (0..n)
            .map(|t| {
                if op_of(t).is_none() {
                    return Vec::new();
                }
                graph.links.iter().filter(|l| l.to.0 == t && l.ty == PortType::Control && op_of(l.from.0).is_some()).map(|l| l.from.0).collect()
            })
            .collect();
        let level = levels(&deps).expect("the graph is acyclic");
        for &i in &graph.order {
            let Some(op) = op_of(i) else { continue };
            let l = level[i];
            if c.levels.len() <= l {
                c.levels.resize_with(l + 1, Vec::new);
            }
            // an LFO level gets all three shapes' batches, each with room for every LFO of the level (a shape change
            // moves one between them without allocating)
            if let Op::Lfo(_) = op {
                if !c.levels[l].iter().any(|b| matches!(b.op, Op::Lfo(_))) {
                    let room = graph.order.iter().filter(|&&j| level[j] == l && graph.nodes[j].kind == Kind::Lfo).count();
                    for s in [Shape::Sine, Shape::Saw, Shape::Square] {
                        c.levels[l].push(Batch::new(Op::Lfo(s), room));
                    }
                }
            }
            let b = match c.levels[l].iter().position(|b| b.op == op) {
                Some(b) => b,
                None => {
                    c.levels[l].push(Batch::new(op, 0));
                    c.levels[l].len() - 1
                }
            };
            let (a, bb) = (c.column(i, 0), c.column(i, 1));
            let batch = &mut c.levels[l][b];
            batch.a.push(a.0, a.1, a.2);
            batch.b.push(bb.0, bb.1, bb.2);
            batch.state.push(if graph.nodes[i].kind == Kind::Lfo { graph.nodes[i].f32("phase", 0.0) } else { 0.0 });
            batch.out.push(c.table.base[i]);
            batch.node.push(i as u32);
            c.at[i] = Some((l, b, batch.out.len() - 1));
        }
        for (node, port, p) in params {
            let default = p.get();
            let (slot, scale, offset) = c.table.inputs[*node][*port].col(default);
            c.param_in.push(slot, scale, offset);
            c.params.push(p.clone());
            c.param_at.push((*node, *port));
            c.param_default.push(default);
        }
        c
    }

    /// A control half without audio – a face's own (no engine: a deck without sound, the tests, the space's mirror).
    pub fn detached(graph: &Graph) -> Control {
        Control::new(graph, &[], Vec::new(), 48_000.0)
    }

    /// how many value slots (slot 0 the constant, then every node's control outputs)
    pub fn slots(&self) -> usize {
        self.values.len()
    }

    /// the engine's: its clock and the frame thread's link
    pub(crate) fn attach(&mut self, beats: BeatReader, feed: Feed) {
        self.beats = Some(beats);
        self.feed = Some(feed);
    }

    /// node `node`'s output `port` takes its value from `p` at every step's start (a module's control output, #342)
    pub(crate) fn feed(&mut self, node: usize, port: usize, p: Param) {
        let slot = self.table.base[node] + port as u32;
        self.values[slot as usize] = p.get();
        self.feeds.push((slot, p));
    }

    /// a `domain: control` module at node `node`, stepped after the levels (#342)
    #[cfg(feature = "modules")]
    pub(crate) fn add_stepper(&mut self, node: usize, stepper: Box<dyn Stepper>) {
        let ins = vec![0.0; self.table.kinds[node].inputs().len()];
        self.steppers.push(Stepped { node, ins, stepper });
    }

    /// The steppers' turn (after a step): each reads its inputs from this block's values, runs, and writes its feeds.
    fn run_steppers(&mut self, frame: u64, frames: usize) {
        for s in &mut self.steppers {
            for (k, x) in s.ins.iter_mut().enumerate() {
                *x = self.table.input_at(&self.values, s.node, k).unwrap_or(0.0);
            }
            s.stepper.step(&s.ins, frame, frames);
        }
    }

    /// node `node`'s input `port` as a column entry (a computed node's default: the catalog's)
    fn column(&self, node: usize, port: usize) -> (u32, f32, f32) {
        let kind = self.table.kinds[node];
        self.table.inputs[node][port].col(default_for(kind, kind.inputs()[port].name))
    }

    /// the input's columns follow a change of its table entry
    fn apply(&mut self, node: usize, port: usize) {
        if let Some((l, b, k)) = self.at[node] {
            let (slot, scale, offset) = self.column(node, port);
            let batch = &mut self.levels[l][b];
            match port {
                0 => batch.a.set(k, slot, scale, offset),
                1 => batch.b.set(k, slot, scale, offset),
                _ => {}
            }
        }
        if let Some(i) = self.param_at.iter().position(|&at| at == (node, port)) {
            let (slot, scale, offset) = self.table.inputs[node][port].col(self.param_default[i]);
            self.param_in.set(i, slot, scale, offset);
        }
    }

    /// Node `node`'s input `port` gets the base `value` (#188, owner 2026-10-31: a linked knob "turnable as the base"):
    /// a linked input is `source · scale + base` – its modulation keeps flowing on top; an unlinked one is the base
    /// alone (a MIDI CC, a 3D button). A send's blend stays on top. False: no such input.
    pub fn set_base(&mut self, node: usize, port: usize, value: f32) -> bool {
        if !self.table.set_base(node, port, value) {
            return false;
        }
        self.apply(node, port);
        true
    }

    /// A value sent from outside onto node `node`'s input `port` (#189 force / blend, #184): the input becomes
    /// `own · (1 − amount) + value · amount` – amount 1 = force (the value alone), 0 = the input as built again; a link
    /// keeps flowing in proportion. False: no such input.
    pub fn send(&mut self, node: usize, port: usize, value: f32, amount: f32) -> bool {
        if !self.table.send(node, port, value, amount) {
            return false;
        }
        self.apply(node, port);
        true
    }

    /// An LFO's shape (the being's waveform button): it moves into its level's batch of that shape, its phase with it.
    /// Never allocates (the batches have room for the level's LFOs). False: not an LFO.
    pub fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        let Some((l, b, k)) = self.at.get(node).copied().flatten() else { return false };
        let Op::Lfo(now) = self.levels[l][b].op else { return false };
        let shape = Shape::of(shape);
        self.table.shapes[node] = Some(shape as usize);
        if now == shape {
            return true;
        }
        let from = &mut self.levels[l][b];
        let (a, bb, state, out) = (from.a.swap_remove(k), from.b.swap_remove(k), from.state.swap_remove(k), from.out.swap_remove(k));
        from.node.swap_remove(k);
        if let Some(&moved) = from.node.get(k) {
            self.at[moved as usize] = Some((l, b, k));
        }
        let t = self.levels[l].iter().position(|x| x.op == Op::Lfo(shape)).expect("an LFO level has every shape's batch");
        let to = &mut self.levels[l][t];
        to.a.push(a.0, a.1, a.2);
        to.b.push(bb.0, bb.1, bb.2);
        to.state.push(state);
        to.out.push(out);
        to.node.push(node as u32);
        self.at[node] = Some((l, t, to.out.len() - 1));
        true
    }

    /// The live context (#409) from now on: the module steppers get it at once, the audio half's module blocks before
    /// their next call (the engine takes it, `take_context`). Never allocates.
    pub fn set_context(&mut self, c: Context) {
        self.context = c;
        self.fresh = true;
        for s in &mut self.steppers {
            s.stepper.context(&c);
        }
    }

    /// the live context last applied (the default before any)
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// a context the audio half has not had yet (the engine hands it to its blocks)
    pub(crate) fn take_context(&mut self) -> Option<Context> {
        std::mem::take(&mut self.fresh).then_some(self.context)
    }

    fn command(&mut self, c: Command) {
        match c {
            Command::Base { node, port, value } => self.set_base(node as usize, port as usize, value),
            Command::Send { node, port, value, amount } => self.send(node as usize, port as usize, value, amount),
            Command::Shape { node, shape } => self.set_lfo_shape(node as usize, shape as usize),
            Command::Context(c) => {
                self.set_context(c);
                true
            }
        };
    }

    /// One step of `dt` seconds at the clock `tempo`: the analyzers (causal: the samples rendered so far), the tempo
    /// node, the levels, the audio nodes' Params.
    pub fn step(&mut self, dt: f32, tempo: &tempo::Now) {
        let v = &mut self.values;
        for (slot, p) in &self.feeds {
            v[*slot as usize] = p.get();
        }
        let base = &self.table.base;
        for (node, a) in &mut self.analyzers {
            let b = base[*node] as usize;
            v[b..b + 4].copy_from_slice(&a.update(dt, 0));
        }
        self.tempo = *tempo;
        if let Some(i) = self.tempo_node {
            let p = &tempo.pos;
            // pulse: 1 on the beat, gone by its middle (0 while stopped)
            let pulse = if tempo.running { (1.0 - p.phase).powi(6) } else { 0.0 };
            let b = base[i] as usize;
            for (o, x) in v[b..b + 6].iter_mut().zip([tempo.bpm, p.beat, p.bar as f64, p.phase, p.bar_phase, pulse]) {
                *o = x as f32;
            }
        }
        for level in &mut self.levels {
            for b in level {
                b.step(v, dt);
            }
        }
        for (i, p) in self.params.iter().enumerate() {
            p.set(self.param_in.get(v, i));
        }
    }

    /// The engine's block (`frame` = its first frame, `rate` the engine's): the frame thread's commands, one step of
    /// `QUANTUM` frames at the clock there, the values into the snapshot ring. Never allocates.
    pub(crate) fn block(&mut self, frame: u64, frames: usize, rate: f32) {
        if let Some(mut feed) = self.feed.take() {
            while let Ok(c) = feed.commands.pop() {
                self.command(c);
            }
            self.feed = Some(feed);
        }
        let now = match self.beats.as_ref().and_then(BeatReader::block) {
            Some((map, _, start)) => tempo::Now { bpm: map.bpm, pos: map.position(start), per_bar: map.per_bar, running: map.running },
            None => tempo::Now::default(),
        };
        match &mut self.cadence {
            None => {
                self.step(frames as f32 / rate, &now);
                self.run_steppers(frame, frames);
            }
            Some((next, period)) => {
                let (due, period) = (*next, *period);
                if frame as f64 >= due {
                    *next = due + period;
                    self.step(period as f32 / rate, &now);
                    self.run_steppers(frame, frames);
                }
            }
        }
        if let Some(feed) = &self.feed {
            feed.snapshots.write(frame, &self.values);
        }
    }

    /// The reference of the P5 golden diff: step at `hz` (the frame thread's rate before #319) instead of every block.
    pub(crate) fn every(&mut self, hz: f32, rate: f32) {
        self.cadence = Some((0.0, f64::from(rate) / f64::from(hz)));
    }

    /// `node`'s control input `port`: the link mapped by it (and the last send), else the node's own param, else
    /// `default`
    pub fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        self.table.input(&self.values, node, port, default)
    }

    /// node `node`'s input `port` by index (an unset one at the catalog's default) – what a parameter path `node.port`
    /// reads (deck-outputs, #282). None = no such input.
    pub fn input_at(&self, node: usize, port: usize) -> Option<f32> {
        self.table.input_at(&self.values, node, port)
    }

    /// `node`'s control outputs this block (its port order)
    pub fn outputs(&self, node: usize) -> &[f32] {
        self.table.outputs(&self.values, node)
    }

    /// a linked input's base (its link's offset – what its knob turns); None for an unlinked one (its value is `input`)
    pub fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.table.base_of(node, port)
    }

    /// an LFO's shape (an index into LFO_SHAPES); None for other nodes
    pub fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.table.shapes.get(node).copied().flatten()
    }

    /// The nodes in scene order: (id, kind, input port names) – the table a host resolves `node.port` names with.
    pub fn nodes(&self) -> impl Iterator<Item = (&str, &'static str, Vec<&'static str>)> + '_ {
        self.table.nodes()
    }

    /// (levels, batches) of the computed nodes – the shape the build found (empty batches not counted)
    pub fn shape(&self) -> (usize, usize) {
        (self.levels.len(), self.levels.iter().flatten().filter(|b| !b.out.is_empty()).count())
    }
}

/// The frame thread's face of the control half: a copy of the input table, the values of the heard block, and the way
/// to the engine (a link: the snapshot ring and the command ring) – or, without an engine, a control half of its own that
/// `update` steps. The picture, the outputs, the space and the pad read and turn it; it never touches the audio thread's
/// state.
pub struct Controls {
    table: Table,
    values: Vec<f32>,
    half: Half,
    /// commands the ring had no room for (sent with the next update)
    waiting: Vec<Command>,
    /// the clock at the heard moment (quantised cuts read it)
    pub tempo: tempo::Now,
    /// the live context last sent (#409): an unchanged one is not sent again
    sent: Option<Context>,
}

enum Half {
    Engine(Link),
    Own(Box<Control>),
}

impl Controls {
    /// The face of the engine's control half (its handles' link), or of its own when there is no engine.
    pub fn new(graph: &Graph, audio: Option<&mut crate::Handles>) -> Controls {
        let table = Table::new(graph);
        let half = match audio.and_then(|h| h.control.take()) {
            Some(link) => Half::Engine(link),
            None => Half::Own(Box::new(Control::detached(graph))),
        };
        Controls { values: vec![0.0; table.slots], table, half, waiting: Vec::new(), tempo: tempo::Now::default(), sent: None }
    }

    /// One video frame: `tempo` = the clock at the heard moment, `lead` = how many frames the engine has rendered past
    /// it. With an engine: the values of the block being heard (the snapshot ring); without: one step of `dt`.
    pub fn update(&mut self, dt: f32, lead: usize, tempo: &tempo::Now) {
        self.tempo = *tempo;
        match &mut self.half {
            Half::Engine(link) => {
                while let Some(&c) = self.waiting.first() {
                    if link.commands.push(c).is_err() {
                        break;
                    }
                    self.waiting.remove(0);
                }
                let heard = link.frames.load(Ordering::Relaxed).saturating_sub(lead as u64);
                link.snapshots.read(heard, &mut self.values);
            }
            Half::Own(c) => {
                c.step(dt, tempo);
                self.values.copy_from_slice(&c.values);
            }
        }
    }

    fn command(&mut self, c: Command) {
        match &mut self.half {
            Half::Engine(link) => {
                if !self.waiting.is_empty() || link.commands.push(c).is_err() {
                    self.waiting.push(c);
                }
            }
            Half::Own(own) => own.command(c),
        }
    }

    /// Set `node`'s control input `port`'s base (`Control::set_base`): a MIDI CC, a 3D button. False: no such input.
    pub fn set_base(&mut self, node: usize, port: usize, value: f32) -> bool {
        if !self.table.set_base(node, port, value) {
            return false;
        }
        self.command(Command::Base { node: node as u32, port: port as u32, value });
        true
    }

    /// Send a value onto `node`'s input `port` (`Control::send`). False: no such input.
    pub fn send(&mut self, node: usize, port: usize, value: f32, amount: f32) -> bool {
        if !self.table.send(node, port, value, amount) {
            return false;
        }
        self.command(Command::Send { node: node as u32, port: port as u32, value, amount });
        true
    }

    /// An LFO's shape from outside (the being's waveform button); its phase carries over. False: not an LFO.
    pub fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        if self.lfo_shape(node).is_none() {
            return false;
        }
        self.table.shapes[node] = Some(Shape::of(shape) as usize);
        self.command(Command::Shape { node: node as u32, shape: shape as u32 });
        true
    }

    /// The live context for the engine's module nodes (#409: the deck's mood, persona and the being's senses – the
    /// platform's frame sends `Deck::context` or its being's): applied at the next block's start; an unchanged one is
    /// not sent again.
    pub fn set_context(&mut self, c: Context) {
        if self.sent == Some(c) {
            return;
        }
        self.sent = Some(c);
        self.command(Command::Context(c));
    }

    pub fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        self.table.input(&self.values, node, port, default)
    }

    pub fn input_at(&self, node: usize, port: usize) -> Option<f32> {
        self.table.input_at(&self.values, node, port)
    }

    pub fn outputs(&self, node: usize) -> &[f32] {
        self.table.outputs(&self.values, node)
    }

    pub fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.table.base_of(node, port)
    }

    pub fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.table.shapes.get(node).copied().flatten()
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&str, &'static str, Vec<&'static str>)> + '_ {
        self.table.nodes()
    }

    /// the first FFT node's values (for the HUD)
    pub fn fft(&self) -> Option<[f32; 4]> {
        let i = self.table.kinds.iter().position(|&k| k == Kind::Fft)?;
        let o = self.outputs(i);
        Some([o[0], o[1], o[2], o[3]])
    }
}

/// the space's view of the control domain (deck-space's `Knobs`, #198: the browser's is the page's mirror)
#[cfg(feature = "space")]
impl deck_space::space::values::Knobs for Controls {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        Controls::input(self, node, port, default)
    }
    fn outputs(&self, node: usize) -> &[f32] {
        Controls::outputs(self, node)
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        Controls::lfo_shape(self, node)
    }
    fn set(&mut self, node: usize, port: usize, value: f32) -> bool {
        Controls::set_base(self, node, port, value)
    }
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        Controls::base(self, node, port)
    }
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        Controls::set_lfo_shape(self, node, shape)
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
