//! The module host (#280, #305, #341): runs rig modules – wasm built against deck-module's SDK – behind a [`Backend`].
//! - [`Loaded::load`] reads a module's descriptor from the `deck.module` section without running anything, checks it
//!   (by its ABI: v1, or v2 with the layout derived from the descriptor) and the backend's imports and exports, and
//!   compiles it once (the backend caches).
//! - An ABI v2 module runs as a [`v2::Instance`] (research/ABI-V2.md §1.1, §6): one copy in and one copy out per block,
//!   the layout check, asset binding, the memory frozen after init, strict and show modes (a show restarts a trapped
//!   instance off the audio thread), [`Preset`]s, [`ModuleStats`].
//! - [`Instance`] is what the engine drives today (its `ModuleInstance` seam): it picks v1 or v2 by `dm_abi()` and
//!   offers the seam's calls on both. v1 (`v1.rs`) stays until the three repository modules move (#343).
//! - The [`Backend`] is the only part that touches a wasm engine: `compile`, `instantiate`, `call`, `write`, `read`,
//!   `set_budget`, `freeze_memory`. The native deck uses [`wasmtime_backend::Wasmtime`]; the conformance suite (tests/)
//!   is generic over it.
use anyhow::{anyhow, bail, Result};
use deck_catalog::{Channels, Descriptor, Dir, Domain, PortType};
use deck_core::tempo::TempoMap;
use deck_module::host::read_descriptor;
use deck_module::v2::{Layout as IoLayout, ParamInfo};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

pub mod preset;
pub mod stats;
mod v1;
pub mod v2;
#[cfg(feature = "wasmtime")]
pub mod wasmtime_backend;

pub use preset::{Preset, PresetReport};
pub use stats::ModuleStats;
pub use v2::{AssetData, Fresh, Mode, Rebuild, Setup};
/// what the engine's event host passes in and out of a v2 module
pub use deck_module::v2::{Ev, Kind as EvKind};

/// a module's linear memory is capped here (this leaves room for a creator's tables and assets)
pub const MEMORY_LIMIT: usize = 64 << 20;
/// log lines a module may print: a burst, then this many a second (the rest is counted, once reported)
const LOG_BURST: f64 = 10.0;
const LOG_PER_SECOND: f64 = 10.0;
/// the one import a module may have
pub const LOG_IMPORT: (&str, &str) = ("env", "deck_log");
/// the exports every ABI v1 module has
pub const REQUIRED_EXPORTS: [&str; 7] = ["memory", "dm_abi", "dm_io", "dm_new", "dm_set_param", "dm_process", "dm_control"];

/// the module's entry points the host calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Export {
    /// `dm_abi() -> u32`
    Abi,
    /// `dm_io() -> u32`: where the io lies in the module's memory
    Io,
    /// `dm_layout() -> u32` (v2): the hash of the module's layout
    Layout,
    /// `dm_new()`: builds the module (runs its `init`)
    New,
    /// `dm_set_param(index: u32, value: f32)` (v1)
    SetParam,
    /// `dm_process(frames: u32)` (v1) · `dm_process()` (v2: the frames are in the header)
    Process,
    /// `dm_control(frames: u32)` (v1) · `dm_control()` (v2)
    Control,
    /// `dm_asset_buf(slot: u32, len: u32) -> u32` (v2, a module with assets): a buffer in the module's heap
    AssetBuf,
}

impl Export {
    pub const ALL: [Export; 8] = [Export::Abi, Export::Io, Export::Layout, Export::New, Export::SetParam, Export::Process, Export::Control, Export::AssetBuf];

    pub fn name(self) -> &'static str {
        match self {
            Export::Abi => "dm_abi",
            Export::Io => "dm_io",
            Export::Layout => "dm_layout",
            Export::New => "dm_new",
            Export::SetParam => "dm_set_param",
            Export::Process => "dm_process",
            Export::Control => "dm_control",
            Export::AssetBuf => "dm_asset_buf",
        }
    }
}

/// an argument of an export call
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arg {
    U32(u32),
    F32(f32),
}

/// why a call did not return
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// the call ran past the budget set with `set_budget`
    Budget,
    /// a trap (a panic is one), the first line of its message
    Trap(String),
}

/// What the host needs of a wasm engine. A backend instance is one running module with its own memory.
pub trait Backend: Sized {
    /// the engine: compiler, clock for the budget, compile cache. Shared by all the modules of one library.
    type Engine: Send + Sync;
    /// a compiled module, cheap to instantiate again
    type Compiled: Send + Sync;

    /// Compiles `wasm` (`hash` = its hex SHA-256, the cache key) → the module and whether it came from the engine's disk cache.
    fn compile(engine: &Self::Engine, wasm: &[u8], hash: &str) -> Result<(Self::Compiled, bool, Duration)>;
    /// the `(module, name)` of every import – the host refuses anything but [`LOG_IMPORT`]
    fn imports(compiled: &Self::Compiled) -> Vec<(String, String)>;
    fn has_export(compiled: &Self::Compiled, name: &str) -> bool;
    /// An instance with its own memory (capped at [`MEMORY_LIMIT`]); `name` labels its log lines. Runs no export (a
    /// start function aside). The log import (`env.deck_log(level, ptr, len)`) is wired to [`log_line`] with a [`LogLimiter`].
    fn instantiate(compiled: &Self::Compiled, name: &str) -> Result<Self>;
    /// Calls an export; the result of those that return a value (`Abi`, `Io`, `Layout`, `AssetBuf`), else 0.
    fn call(&mut self, export: Export, args: &[Arg]) -> Result<u32, Fault>;
    /// the memory's size in bytes now
    fn memory_size(&self) -> usize;
    /// writes `bytes` at the absolute memory offset `off` (in range: the host checked the io lies in the memory)
    fn write(&mut self, off: usize, bytes: &[u8]);
    fn read(&self, off: usize, out: &mut [u8]);
    /// the time the next `call` may take, in milliseconds
    fn set_budget(&mut self, ms: u64);
    /// From now on the memory may not grow past its size: `memory.grow` fails (-1), it does not trap (ABI-V2 §6 –
    /// the host calls it after `dm_new` and the assets, so nothing maps memory on the audio thread).
    fn freeze_memory(&mut self);
    /// a monotonic clock in microseconds (the cpu meter; the host itself uses no clock)
    fn clock_us(&self) -> u64;
}

/// where a graph input reads from: the n-th declared audio / control input port of the module, or its parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InBind {
    Audio(usize),
    Control(usize),
    Param(usize),
}

/// what a graph output carries: the module's n-th audio output port, or its n-th control output (ABI v2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBind {
    Audio(usize),
    Control(usize),
}

/// One graph port of a module: its name, whether it carries audio (else a control value) and – an ABI v2 audio port –
/// its channels (None: ABI v1's stereo pair). What the platform registers as the node kind's port (deck-space modreg).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPort {
    pub name: String,
    pub audio: bool,
    pub channels: Option<Channels>,
}

/// One event port of an ABI v2 module: the deck-events names it takes / emits, its props in slot order, its capacity.
#[derive(Debug, Clone, PartialEq)]
pub struct EventPort {
    pub name: String,
    pub names: Vec<String>,
    pub props: Vec<String>,
    pub capacity: u16,
}

/// How a module's descriptor maps onto the graph (#342): inputs are the declared audio inputs, then the control inputs
/// (each in declaration order), then the parameters (every parameter is also a control input); outputs are the declared
/// audio outputs and – ABI v2 – the control outputs, in declaration order (a control output with a `path` is a graph
/// output too: deck-space binds it to its target). Event ports are not graph ports: the engine's event host routes them
/// by their deck-events names. `in_ports` / `out_ports` name the graph ports (parallel to `inputs` / `outputs`).
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub inputs: Vec<InBind>,
    pub outputs: Vec<OutBind>,
    pub in_ports: Vec<GraphPort>,
    pub out_ports: Vec<GraphPort>,
    pub events_in: Vec<EventPort>,
    pub events_out: Vec<EventPort>,
    /// (graph output, path, mode) per control output bound to a target (ABI-V2 §5)
    pub targets: Vec<(usize, String, deck_catalog::Mode)>,
    /// `audio` runs in the engine's audio half (`dm_process`), `control` in its control half (`dm_control`)
    pub domain: Domain,
}

impl Layout {
    /// Err when the module needs something this host cannot do yet (video; a v1 module's control outputs)
    pub fn of(d: &Descriptor) -> Result<Layout> {
        let v2 = d.abi == Some(deck_module::v2::ABI_VERSION);
        let mut l = Layout {
            inputs: Vec::new(),
            outputs: Vec::new(),
            in_ports: Vec::new(),
            out_ports: Vec::new(),
            events_in: Vec::new(),
            events_out: Vec::new(),
            targets: Vec::new(),
            domain: if v2 { d.domain } else { Domain::Audio },
        };
        // an audio port's channels: v1 the stereo pair (None); v2 as declared, a cv port and an undeclared one 2
        let channels = |p: &deck_catalog::PortDesc| match (v2, p.ty, p.channels) {
            (false, ..) => None,
            (true, PortType::Cv, _) | (true, _, None) => Some(Channels::Count(2)),
            (true, _, c) => c,
        };
        let port = |p: &deck_catalog::PortDesc, audio: bool| GraphPort { name: p.name.to_string(), audio, channels: if audio { channels(p) } else { None } };
        let (mut audio_in, mut control_in) = (Vec::new(), Vec::new());
        let (mut audio_out, mut control_out) = (0, 0);
        for p in d.ports.iter() {
            match (p.dir, p.ty) {
                (_, PortType::Video) => bail!("module '{}': video port '{}' – a video module is not supported by this host yet (#185)", d.name, p.name),
                (Dir::Out, PortType::Control) if !v2 => {
                    bail!("module '{}': control output '{}' – control outputs of v1 modules are not wired into the graph", d.name, p.name)
                }
                (Dir::In, PortType::Audio | PortType::Cv) => audio_in.push(port(p, true)),
                (Dir::In, PortType::Control) => control_in.push(port(p, false)),
                (Dir::Out, PortType::Audio | PortType::Cv) => {
                    l.outputs.push(OutBind::Audio(audio_out));
                    l.out_ports.push(port(p, true));
                    audio_out += 1;
                }
                (Dir::Out, PortType::Control) => {
                    if !p.path.is_empty() {
                        l.targets.push((l.outputs.len(), p.path.to_string(), p.mode));
                    }
                    l.outputs.push(OutBind::Control(control_out));
                    l.out_ports.push(port(p, false));
                    control_out += 1;
                }
                (dir, PortType::Event) if v2 => {
                    let e = EventPort {
                        name: p.name.to_string(),
                        names: p.names.iter().map(|n| n.to_string()).collect(),
                        props: p.props.iter().map(|n| n.to_string()).collect(),
                        capacity: p.capacity,
                    };
                    if dir == Dir::In { l.events_in.push(e) } else { l.events_out.push(e) }
                }
                (_, PortType::Event) => {}
            }
        }
        l.inputs.extend((0..audio_in.len()).map(InBind::Audio));
        l.inputs.extend((0..control_in.len()).map(InBind::Control));
        l.inputs.extend((0..d.params.len()).map(InBind::Param));
        l.in_ports.extend(audio_in);
        l.in_ports.extend(control_in);
        l.in_ports.extend(d.params.iter().map(|p| GraphPort { name: p.name.to_string(), audio: false, channels: None }));
        Ok(l)
    }
}

/// What a v2 module adds to its `Loaded`: the io layout derived from its descriptor, the parameters' ranges, the
/// factory preset resolved.
#[derive(Debug, Clone)]
pub struct V2Loaded {
    pub io: IoLayout,
    pub params: Vec<ParamInfo>,
    pub factory: Option<Preset>,
}

/// A loaded module: descriptor read, imports and exports checked, compiled. Cheap to share.
pub struct Loaded<B: Backend> {
    pub desc: Descriptor,
    pub layout: Layout,
    /// hex SHA-256 of the wasm
    pub hash: String,
    pub compile: Duration,
    pub from_disk: bool,
    /// the ABI the descriptor declares (1 or 2)
    pub abi: u32,
    /// an ABI 2 module's layout and parameters
    pub v2: Option<V2Loaded>,
    pub(crate) compiled: B::Compiled,
}

impl<B: Backend> std::fmt::Debug for Loaded<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Loaded({} {} abi {} {})", self.desc.name, self.desc.version, self.abi, &self.hash[..12])
    }
}

impl<B: Backend> Loaded<B> {
    /// the descriptor first (nothing runs), then the compile (cached by the backend), then the sandbox check
    pub fn load(engine: &B::Engine, wasm: &[u8]) -> Result<Loaded<B>> {
        let hash = hex(&Sha256::digest(wasm));
        let json = read_descriptor(wasm).ok_or_else(|| anyhow!("no `deck.module` section – the wasm was not built with deck-module"))?;
        let desc: Descriptor = serde_json::from_str(json).map_err(|e| anyhow!("descriptor: {e}"))?;
        let abi = desc.abi.unwrap_or(0);
        let (v2, required): (Option<V2Loaded>, Vec<&str>) = if abi == deck_module::v2::ABI_VERSION {
            let shape = deck_module::host::validate_v2(&desc).map_err(|e| anyhow!(e))?;
            let io = IoLayout::of(&shape);
            v2::assets_fit(&desc, &io)?;
            let params = desc.params.iter().map(|p| ParamInfo { min: p.min, max: p.max, default: p.default.unwrap_or(p.min) }).collect();
            let factory = match Preset::factory(&desc) {
                Some((p, report)) if report.is_clean() => Some(p),
                Some((_, report)) => bail!("module '{}': its factory preset names {:?} / sets {:?} out of range", desc.name, report.unknown, report.out_of_range),
                None => None,
            };
            let call = if desc.domain == deck_catalog::Domain::Control { "dm_control" } else { "dm_process" };
            let mut required = vec!["memory", "dm_abi", "dm_io", "dm_layout", "dm_new", call];
            if !desc.assets.is_empty() {
                required.push("dm_asset_buf");
            }
            (Some(V2Loaded { io, params, factory }), required)
        } else {
            deck_module::host::validate(&desc).map_err(|e| anyhow!(e))?;
            (None, REQUIRED_EXPORTS.to_vec())
        };
        let layout = Layout::of(&desc)?;
        let (compiled, from_disk, compile) = B::compile(engine, wasm, &hash)?;
        for (m, n) in B::imports(&compiled) {
            if (m.as_str(), n.as_str()) != LOG_IMPORT {
                bail!("module '{}' imports {m}.{n} – a module may import only env.deck_log", desc.name);
            }
        }
        for export in required {
            if !B::has_export(&compiled, export) {
                bail!("module '{}' does not export `{export}`", desc.name);
            }
        }
        Ok(Loaded { desc, layout, hash, compile, from_disk, abi, v2, compiled })
    }

    /// the slot of a declared variable (`reads`)
    pub fn var_slot(&self, key: &str) -> Option<usize> {
        self.desc.reads.iter().position(|k| k == key)
    }

    /// the slot of a declared asset
    pub fn asset_slot(&self, name: &str) -> Option<usize> {
        self.desc.assets.iter().position(|a| a.name == name)
    }

    /// a parameter's index by name
    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.desc.params.iter().position(|p| p.name == name)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// a token bucket for `log!` (per module)
pub struct LogLimiter {
    tokens: f64,
    last: Option<f64>,
    /// lines refused since the last one let through
    pub dropped: u64,
}

impl Default for LogLimiter {
    fn default() -> Self {
        LogLimiter { tokens: LOG_BURST, last: None, dropped: 0 }
    }
}

impl LogLimiter {
    /// whether a line at `now` (seconds on any monotonic clock) may be printed; `dropped` counts the ones that may not
    pub fn allow(&mut self, now: f64) -> bool {
        if let Some(last) = self.last {
            self.tokens = (self.tokens + (now - last).max(0.0) * LOG_PER_SECOND).min(LOG_BURST);
        }
        self.last = Some(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            self.dropped += 1;
            false
        }
    }
}

/// what a backend does with a module's `deck_log` call: limit, report the dropped ones once, print
pub fn log_line(limiter: &mut LogLimiter, now: f64, name: &str, level: u32, text: &str) {
    let dropped = limiter.dropped;
    if !limiter.allow(now) {
        return;
    }
    if dropped > 0 {
        tracing::warn!("module '{name}': {dropped} log lines dropped (rate limit)");
        limiter.dropped = 0;
    }
    match level {
        0 => tracing::info!("module '{name}': {text}"),
        1 => tracing::warn!("module '{name}': {text}"),
        _ => tracing::error!("module '{name}': {text}"),
    }
}

// one per module node, never in a hot array; v1 goes with #343
#[allow(clippy::large_enum_variant)]
enum Inner<B: Backend> {
    V1(v1::Instance<B>),
    V2(v2::Instance<B>),
}

/// One running module behind the engine's seam (`deck-engine`'s `ModuleInstance`): v1 or v2, picked by `dm_abi()`.
/// A v2 module runs strict here; [`Instance::v2_mut`] reaches everything else of it (events, control outputs,
/// sensors, assets, presets, stats, a show's restart).
pub struct Instance<B: Backend>(Inner<B>);

impl<B: Backend> Instance<B> {
    /// Instantiates `loaded` for `rate` Hz with `seed` (the same seed plays the same on every deck). The module is built
    /// with its parameters at their defaults; tempo 120 bpm, 4 a bar, from frame 0 until `set_tempo`.
    pub fn new(loaded: &Arc<Loaded<B>>, rate: f64, seed: u64) -> Result<Instance<B>> {
        if loaded.v2.is_some() {
            // `boot` checks `dm_abi() == 2`
            return v2::Instance::new(loaded, Setup::new(rate, seed)).map(|i| Instance(Inner::V2(i)));
        }
        let name = &loaded.desc.name;
        let mut backend = B::instantiate(&loaded.compiled, name)?;
        backend.set_budget(1000);
        let abi = backend.call(Export::Abi, &[]).map_err(|f| anyhow!("{name}: dm_abi: {}", fault_text(&f)))?;
        if abi != deck_module::ABI_VERSION {
            bail!("module '{name}' speaks ABI {abi}, its descriptor says {}", loaded.abi);
        }
        v1::Instance::new(loaded, backend, rate, seed).map(|i| Instance(Inner::V1(i)))
    }

    /// a v2 instance built with its own [`Setup`] (a show, assets) behind the seam
    pub fn from_v2(i: v2::Instance<B>) -> Instance<B> {
        Instance(Inner::V2(i))
    }

    pub fn v2(&self) -> Option<&v2::Instance<B>> {
        match &self.0 {
            Inner::V2(i) => Some(i),
            Inner::V1(_) => None,
        }
    }

    pub fn v2_mut(&mut self) -> Option<&mut v2::Instance<B>> {
        match &mut self.0 {
            Inner::V2(i) => Some(i),
            Inner::V1(_) => None,
        }
    }

    fn loaded(&self) -> &Arc<Loaded<B>> {
        match &self.0 {
            Inner::V1(i) => &i.loaded,
            Inner::V2(i) => i.loaded(),
        }
    }

    pub fn descriptor(&self) -> &Descriptor {
        &self.loaded().desc
    }

    pub fn layout(&self) -> &Layout {
        &self.loaded().layout
    }

    /// the backend (tests, meters); `None` while a v2 instance restarts
    pub fn backend(&self) -> Option<&B> {
        match &self.0 {
            Inner::V1(i) => Some(&i.backend),
            Inner::V2(i) => i.backend(),
        }
    }

    /// why this instance is silent for good (a trap, a budget overrun – strict), if it is
    pub fn silenced(&self) -> Option<&str> {
        match &self.0 {
            Inner::V1(i) => i.silenced.as_deref(),
            Inner::V2(i) => i.silenced(),
        }
    }

    /// microseconds the last call took, and the slowest one
    pub fn cpu_us(&self) -> (u32, u32) {
        match &self.0 {
            Inner::V1(i) => (i.last_us, i.max_us),
            Inner::V2(i) => i.cpu_us(),
        }
    }

    pub fn set_tempo(&mut self, m: &TempoMap) {
        match &mut self.0 {
            Inner::V1(i) => i.set_tempo(m),
            Inner::V2(i) => i.set_tempo(m),
        }
    }

    /// the character the module plays for (empty = none)
    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        match &mut self.0 {
            Inner::V1(i) => i.set_character(id, persona, mood),
            Inner::V2(i) => i.set_character(id, persona, mood),
        }
    }

    /// scene and world variables, key → number (v1: at most 32; v2: the declared `reads`)
    pub fn set_vars(&mut self, vars: &[(String, f32)]) {
        match &mut self.0 {
            Inner::V1(i) => i.set_vars(vars),
            Inner::V2(i) => i.set_vars(vars),
        }
    }

    /// parameter `index` (v1: clamped by the module; v2: by the host); a silenced instance ignores it
    pub fn set_param(&mut self, index: usize, value: f32) {
        match &mut self.0 {
            Inner::V1(i) => i.set_param(index, value),
            Inner::V2(i) => i.set_param(index, value),
        }
    }

    /// the n-th declared control input's value for the next call
    pub fn set_control_in(&mut self, port: usize, value: f32) {
        match &mut self.0 {
            Inner::V1(i) => i.set_control_in(port, value),
            Inner::V2(i) => i.set_control_in(port, value),
        }
    }

    /// the n-th declared audio input for the next call (v2: channels 0 and 1, at most `QUANTUM` frames)
    pub fn set_audio_in(&mut self, port: usize, l: &[f32], r: &[f32]) {
        match &mut self.0 {
            Inner::V1(i) => i.set_audio_in(port, l, r),
            Inner::V2(i) => i.set_audio_in(port, l, r),
        }
    }

    /// the audio output `port` of the last call into `l` / `r` (their lengths = the frames asked)
    pub fn read_audio_out(&mut self, port: usize, l: &mut [f32], r: &mut [f32]) {
        match &mut self.0 {
            Inner::V1(i) => i.read_audio_out(port, l, r),
            Inner::V2(i) => i.read_audio_out(port, l, r),
        }
    }

    /// Audio input `port` for the next call as channel slices (#342: the engine's planar cable, any width; empty =
    /// nothing linked). v2: the port takes `chans.len()` channels (at most what it reserves; the rest of its region is
    /// zero) and says so in its header; v1: channels 0 and 1 (a mono cable on both).
    pub fn set_audio_chans(&mut self, port: usize, chans: &[&[f32]]) {
        match &mut self.0 {
            Inner::V1(i) => match chans {
                [] => i.set_audio_in(port, &[0.0; deck_module::abi::MAX_BLOCK], &[0.0; deck_module::abi::MAX_BLOCK]),
                [m] => i.set_audio_in(port, m, m),
                [l, r, ..] => i.set_audio_in(port, l, r),
            },
            Inner::V2(i) => {
                let reserved = i.io_layout().shape.audio_in.get(port).copied().unwrap_or(0) as usize;
                if reserved == 0 {
                    return;
                }
                if chans.is_empty() {
                    i.set_in_silent(port, true);
                    i.set_channels(true, port, 0);
                    return;
                }
                for c in 0..reserved {
                    let dst = i.audio_in_mut(port, c);
                    match chans.get(c) {
                        Some(src) => {
                            let n = src.len().min(dst.len());
                            dst[..n].copy_from_slice(&src[..n]);
                            dst[n..].fill(0.0);
                        }
                        None => dst.fill(0.0),
                    }
                }
                i.set_channels(true, port, chans.len().min(reserved) as u8);
                i.set_in_silent(port, false);
            }
        }
    }

    /// Channel `c` of audio output `port` of the last call into `dst` (its length = the frames asked); a channel the
    /// output lacks is silent (v1: the stereo pair).
    pub fn read_audio_chan(&mut self, port: usize, c: usize, dst: &mut [f32]) {
        match &mut self.0 {
            Inner::V1(i) => i.read_audio_chan(port, c, dst),
            Inner::V2(i) => {
                let src = i.audio_out(port);
                if c < src.ch() {
                    let s = src.chan(c);
                    let n = dst.len().min(s.len());
                    dst[..n].copy_from_slice(&s[..n]);
                    dst[n..].fill(0.0);
                } else {
                    dst.fill(0.0);
                }
            }
        }
    }

    /// v2: the channels the engine's cable takes out of audio output `port` (an `any` output follows it; 0 = declared)
    pub fn set_out_channels(&mut self, port: usize, ch: usize) {
        if let Inner::V2(i) = &mut self.0 {
            i.set_channels(false, port, ch.min(u8::MAX as usize) as u8);
        }
    }

    /// control output `i` of the last call and whether it was written (unwritten = the consumer holds; v1: none)
    pub fn control_out(&self, i: usize) -> (f32, bool) {
        match &self.0 {
            Inner::V1(_) => (0.0, false),
            Inner::V2(v) => v.control_out(i),
        }
    }

    /// event input `p`'s records for the next call (v2; v1 has no event input) – how many were taken
    pub fn set_events_in(&mut self, p: usize, events: &[Ev]) -> usize {
        match &mut self.0 {
            Inner::V1(_) => 0,
            Inner::V2(i) => i.set_events_in(p, events),
        }
    }

    /// `n` of the next call's input events came late (v2)
    pub fn add_late(&mut self, n: u32) {
        if let Inner::V2(i) = &mut self.0 {
            i.add_late(n);
        }
    }

    /// event output `p`'s records of the last call (v2; v1's events go with #343)
    pub fn events_out(&self, p: usize) -> &[Ev] {
        match &self.0 {
            Inner::V1(_) => &[],
            Inner::V2(i) => i.events_out(p),
        }
    }

    /// v2: the next call swaps a rebuilt instance in (a show's restart) – the event host re-sends the held notes with it
    pub fn swapping(&self) -> bool {
        self.v2().is_some_and(|i| i.swapping())
    }

    /// v2 in a show: the rebuild of a trapped instance once its pause is over ([`v2::Instance::take_rebuild`])
    pub fn take_rebuild(&mut self) -> Option<Rebuild<B>> {
        self.v2_mut().and_then(|i| i.take_rebuild())
    }

    /// v2: a rebuild's result ([`v2::Instance::offer`])
    pub fn offer(&mut self, fresh: Fresh<B>) {
        if let Some(i) = self.v2_mut() {
            i.offer(fresh);
        }
    }

    /// v2: the instance's counters
    pub fn stats(&self) -> Option<&ModuleStats> {
        self.v2().map(|i| i.stats())
    }

    /// One audio call of `frames` frames (v1: ≤ 1024; v2: ≤ `QUANTUM`, the module's one call) starting at `frame`.
    /// False when the instance is (now) silent.
    pub fn process(&mut self, frame: f64, frames: u32) -> bool {
        match &mut self.0 {
            Inner::V1(i) => i.call(frame, frames, false),
            Inner::V2(i) => i.run(frame, frames),
        }
    }

    /// One control tick (v1's `dm_control`; v2: the module's one call)
    pub fn control(&mut self, frame: f64, frames: u32) -> bool {
        match &mut self.0 {
            Inner::V1(i) => i.call(frame, frames, true),
            Inner::V2(i) => i.run(frame, frames),
        }
    }

    /// the events the last call emitted (v1; allocates – for tests and non-realtime consumers)
    pub fn events(&mut self) -> Vec<deck_module::abi::EventRec> {
        match &mut self.0 {
            Inner::V1(i) => i.events(),
            Inner::V2(_) => Vec::new(),
        }
    }

    /// v1: events past the buffer in the last call
    pub fn events_dropped(&mut self) -> u32 {
        match &mut self.0 {
            Inner::V1(i) => i.events_dropped(),
            Inner::V2(i) => i.out_hdr().dropped.iter().sum(),
        }
    }

    /// a character's id as the module reads it (v1 tests)
    pub fn character_id(&mut self) -> String {
        match &mut self.0 {
            Inner::V1(i) => i.character_id(),
            Inner::V2(_) => String::new(),
        }
    }
}

pub(crate) fn fault_text(f: &Fault) -> String {
    match f {
        Fault::Budget => "ran past its time budget".into(),
        Fault::Trap(w) => w.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_limiter_lets_a_burst_through_then_a_steady_rate() {
        let mut l = LogLimiter::default();
        let allowed = (0..50).filter(|_| l.allow(0.0)).count();
        assert_eq!(allowed, 10, "the burst");
        assert_eq!(l.dropped, 40);
        // half a second later: five more
        assert_eq!((0..50).filter(|_| l.allow(0.5)).count(), 5);
        // a long quiet time refills the burst, no more
        assert_eq!((0..50).filter(|_| l.allow(60.5)).count(), 10);
    }

    #[test]
    fn the_layout_maps_ports_then_params_and_refuses_what_it_cannot_run() {
        let d = |abi: u32, ports: &str| {
            let json = format!(
                "{{\"abi\":{abi},\"name\":\"m\",\"version\":\"1\",\"params\":[{{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":0}}],\"ports\":[{ports}]}}"
            );
            if abi == 1 {
                deck_module::host::parse(&json).unwrap()
            } else {
                deck_module::host::parse_v2(&json).unwrap()
            }
        };
        let ok = d(1, "{\"name\":\"x\",\"dir\":\"in\",\"kind\":\"audio\"},{\"name\":\"c\",\"dir\":\"in\",\"kind\":\"control\"},{\"name\":\"o\",\"dir\":\"out\",\"kind\":\"audio\"},{\"name\":\"e\",\"dir\":\"out\",\"kind\":\"event\"}");
        let l = Layout::of(&ok).unwrap();
        assert_eq!(l.inputs, [InBind::Audio(0), InBind::Control(0), InBind::Param(0)]);
        assert_eq!(l.outputs, [OutBind::Audio(0)]);
        assert_eq!(l.in_ports.iter().map(|p| (p.name.as_str(), p.audio, p.channels)).collect::<Vec<_>>(), [("x", true, None), ("c", false, None), ("a", false, None)]);
        assert!(l.events_out.is_empty(), "a v1 module's events stay unwired (it goes with #343)");
        assert!(Layout::of(&d(1, "{\"name\":\"v\",\"dir\":\"out\",\"kind\":\"video\"}")).unwrap_err().to_string().contains("video"));
        assert!(Layout::of(&d(1, "{\"name\":\"v\",\"dir\":\"out\",\"kind\":\"control\"}")).unwrap_err().to_string().contains("control output"));
        // v2 (#342): audio inputs before control inputs, control outputs are graph outputs (a `path` one is a target
        // too), event ports are the event host's, channels as declared
        let v2 = d(
            2,
            "{\"name\":\"k\",\"dir\":\"in\",\"type\":\"control\"},{\"name\":\"w\",\"dir\":\"in\",\"type\":\"audio\",\"channels\":5},\
             {\"name\":\"o\",\"dir\":\"out\",\"type\":\"audio\",\"channels\":\"any\"},{\"name\":\"t\",\"dir\":\"out\",\"type\":\"control\",\"path\":\"fx.gain\",\"mode\":\"mul\"},\
             {\"name\":\"n\",\"dir\":\"in\",\"type\":\"event\",\"names\":[\"music.note\"],\"props\":[\"pitch\"],\"capacity\":16}",
        );
        let l = Layout::of(&v2).unwrap();
        assert_eq!(l.inputs, [InBind::Audio(0), InBind::Control(0), InBind::Param(0)]);
        assert_eq!(l.in_ports[0].channels, Some(Channels::Count(5)));
        assert_eq!(l.outputs, [OutBind::Audio(0), OutBind::Control(0)]);
        assert_eq!(l.out_ports[0].channels, Some(Channels::Any(deck_catalog::AnyChannels::Any)));
        assert_eq!(l.targets, [(1, "fx.gain".to_string(), deck_catalog::Mode::Mul)]);
        assert_eq!(l.events_in, [EventPort { name: "n".into(), names: vec!["music.note".into()], props: vec!["pitch".into()], capacity: 16 }]);
        assert_eq!(l.domain, Domain::Audio);
    }
}
