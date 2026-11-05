//! The module host (#280, #305, #341): runs rig modules – wasm built against deck-module's SDK – behind a [`Backend`].
//! - [`Loaded::load`] reads a module's descriptor from the `deck.module` section without running anything, checks it
//!   (module ABI v2: the layout derived from the descriptor) and the backend's imports and exports, and compiles it
//!   once (the backend caches). A module built against another ABI is refused – it is recompiled (#343, no
//!   compatibility).
//! - A module runs as an [`Instance`] (research/ABI-V2.md §1.1, §6): one copy in and one copy out per block, the
//!   layout check, asset binding, the memory frozen after init, strict and show modes (a show restarts a trapped
//!   instance off the audio thread), [`Preset`]s, [`ModuleStats`]; the engine drives it through its `ModuleInstance`
//!   seam (deck-engine).
//! - The [`Backend`] is the only part that touches a wasm engine: `compile`, `instantiate`, `call`, `write`, `read`,
//!   `set_budget`, `freeze_memory`. The native deck uses [`wasmtime_backend::Wasmtime`]; the conformance suite (tests/)
//!   is generic over it.
use anyhow::{anyhow, bail, Result};
use deck_catalog::{Descriptor, Dir, Domain, PortType};
use deck_module::host::read_descriptor;
use deck_module::{Layout as IoLayout, ParamInfo};
use sha2::{Digest, Sha256};
use std::time::Duration;

mod instance;
pub mod preset;
pub mod stats;
#[cfg(feature = "wasmtime")]
pub mod wasmtime_backend;

pub use instance::{AssetData, Fresh, Instance, Mode, Rebuild, Setup, UNBUDGETED_MS};
pub use preset::{Preset, PresetReport};
pub use stats::ModuleStats;
/// what the engine's event host passes in and out of a module
pub use deck_module::{Ev, Kind as EvKind};

/// a module's linear memory is capped here (this leaves room for a creator's tables and assets)
pub const MEMORY_LIMIT: usize = 64 << 20;
/// log lines a module may print: a burst, then this many a second (the rest is counted, once reported)
const LOG_BURST: f64 = 10.0;
const LOG_PER_SECOND: f64 = 10.0;
/// the one import a module may have
pub const LOG_IMPORT: (&str, &str) = ("env", "deck_log");

/// the module's entry points the host calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Export {
    /// `dm_abi() -> u32`
    Abi,
    /// `dm_io() -> u32`: where the io lies in the module's memory
    Io,
    /// `dm_layout() -> u32`: the hash of the module's layout
    Layout,
    /// `dm_new()`: builds the module (runs its `init`)
    New,
    /// `dm_process()` (the frames are in the header)
    Process,
    /// `dm_control()`
    Control,
    /// `dm_asset_buf(slot: u32, len: u32) -> u32` (a module with assets): a buffer in the module's heap
    AssetBuf,
}

impl Export {
    pub const ALL: [Export; 7] = [Export::Abi, Export::Io, Export::Layout, Export::New, Export::Process, Export::Control, Export::AssetBuf];

    pub fn name(self) -> &'static str {
        match self {
            Export::Abi => "dm_abi",
            Export::Io => "dm_io",
            Export::Layout => "dm_layout",
            Export::New => "dm_new",
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

/// what a graph output carries: the module's n-th audio output port, or its n-th control output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBind {
    Audio(usize),
    Control(usize),
}

/// One graph port of a module (the catalog's: `Descriptor::graph` numbers them). What the platform registers as the node
/// kind's port (deck-space modreg).
pub use deck_catalog::GraphPort;

/// One event port of a module: the deck-events names it takes / emits, its props in slot order, its capacity.
#[derive(Debug, Clone, PartialEq)]
pub struct EventPort {
    pub name: String,
    pub names: Vec<String>,
    pub props: Vec<String>,
    pub capacity: u16,
}

/// How a module's descriptor maps onto the graph (#342): inputs are the declared audio inputs, then the control inputs
/// (each in declaration order), then the parameters (every parameter is also a control input); outputs are the declared
/// audio outputs and the control outputs, in declaration order (a control output with a `path` is a graph
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
    /// Err when the module needs something this host cannot do yet (video)
    pub fn of(d: &Descriptor) -> Result<Layout> {
        // the graph's numbering is the catalog's (one rule for the binds here and the graph's registration)
        let graph = d.graph();
        let mut l = Layout {
            inputs: Vec::new(),
            outputs: Vec::new(),
            in_ports: graph.inputs,
            out_ports: graph.outputs,
            events_in: Vec::new(),
            events_out: Vec::new(),
            targets: graph.targets,
            domain: d.domain,
        };
        let (mut audio_in, mut control_in) = (0, 0);
        let (mut audio_out, mut control_out) = (0, 0);
        for p in d.ports.iter() {
            match (p.dir, p.ty) {
                (_, PortType::Video) => bail!("module '{}': video port '{}' – a video module is not supported by this host yet (#185)", d.name, p.name),
                (Dir::In, PortType::Audio | PortType::Cv) => audio_in += 1,
                (Dir::In, PortType::Control) => control_in += 1,
                (Dir::Out, PortType::Audio | PortType::Cv) => {
                    l.outputs.push(OutBind::Audio(audio_out));
                    audio_out += 1;
                }
                (Dir::Out, PortType::Control) => {
                    l.outputs.push(OutBind::Control(control_out));
                    control_out += 1;
                }
                (dir, PortType::Event) => {
                    let e = EventPort {
                        name: p.name.to_string(),
                        names: p.names.iter().map(|n| n.to_string()).collect(),
                        props: p.props.iter().map(|n| n.to_string()).collect(),
                        capacity: p.capacity,
                    };
                    if dir == Dir::In { l.events_in.push(e) } else { l.events_out.push(e) }
                }
            }
        }
        l.inputs.extend((0..audio_in).map(InBind::Audio));
        l.inputs.extend((0..control_in).map(InBind::Control));
        l.inputs.extend((0..d.params.len()).map(InBind::Param));
        debug_assert!(l.inputs.len() == l.in_ports.len() && l.outputs.len() == l.out_ports.len(), "the binds follow the catalog's graph ports");
        Ok(l)
    }
}

/// A loaded module: descriptor read, imports and exports checked, compiled. Cheap to share.
pub struct Loaded<B: Backend> {
    pub desc: Descriptor,
    pub layout: Layout,
    /// hex SHA-256 of the wasm
    pub hash: String,
    pub compile: Duration,
    pub from_disk: bool,
    /// the module's io layout, derived from its descriptor (the SDK's `Layout::of`)
    pub io: IoLayout,
    /// the parameters' ranges and defaults, by index
    pub params: Vec<ParamInfo>,
    /// the descriptor's factory preset, resolved
    pub factory: Option<Preset>,
    pub(crate) compiled: B::Compiled,
}

impl<B: Backend> std::fmt::Debug for Loaded<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Loaded({} {} {})", self.desc.name, self.desc.version, &self.hash[..12])
    }
}

impl<B: Backend> Loaded<B> {
    /// the descriptor first (nothing runs), then the compile (cached by the backend), then the sandbox check
    pub fn load(engine: &B::Engine, wasm: &[u8]) -> Result<Loaded<B>> {
        let hash = hex(&Sha256::digest(wasm));
        let json = read_descriptor(wasm).ok_or_else(|| anyhow!("no `deck.module` section – the wasm was not built with deck-module"))?;
        let desc: Descriptor = serde_json::from_str(json).map_err(|e| anyhow!("descriptor: {e}"))?;
        let shape = deck_module::host::validate(&desc).map_err(|e| anyhow!(e))?;
        let io = IoLayout::of(&shape);
        instance::assets_fit(&desc, &io)?;
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
        Ok(Loaded { desc, layout, hash, compile, from_disk, io, params, factory, compiled })
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

pub(crate) fn fault_text(f: &Fault) -> String {
    match f {
        Fault::Budget => "ran past its time budget".into(),
        Fault::Trap(w) => w.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_catalog::Channels;

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
        let d = |ports: &str| {
            let json = format!("{{\"abi\":2,\"name\":\"m\",\"version\":\"1\",\"params\":[{{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":0}}],\"ports\":[{ports}]}}");
            deck_module::host::parse(&json).unwrap()
        };
        assert!(Layout::of(&d("{\"name\":\"v\",\"dir\":\"out\",\"type\":\"video\"}")).unwrap_err().to_string().contains("video"));
        // #342: audio inputs before control inputs, control outputs are graph outputs (a `path` one is a target too),
        // event ports are the event host's, channels as declared (a cv port and an undeclared one: 2)
        let l = Layout::of(&d(
            "{\"name\":\"k\",\"dir\":\"in\",\"type\":\"control\"},{\"name\":\"w\",\"dir\":\"in\",\"type\":\"audio\",\"channels\":5},\
             {\"name\":\"o\",\"dir\":\"out\",\"type\":\"audio\",\"channels\":\"any\"},{\"name\":\"t\",\"dir\":\"out\",\"type\":\"control\",\"path\":\"fx.gain\",\"mode\":\"mul\"},\
             {\"name\":\"n\",\"dir\":\"in\",\"type\":\"event\",\"names\":[\"music.note\"],\"props\":[\"pitch\"],\"capacity\":16},\
             {\"name\":\"c\",\"dir\":\"out\",\"type\":\"cv\"}",
        ))
        .unwrap();
        assert_eq!(l.inputs, [InBind::Audio(0), InBind::Control(0), InBind::Param(0)]);
        assert_eq!(l.in_ports.iter().map(|p| (p.name.as_str(), p.audio)).collect::<Vec<_>>(), [("w", true), ("k", false), ("a", false)]);
        assert_eq!(l.in_ports[0].channels, Some(Channels::Count(5)));
        assert_eq!(l.outputs, [OutBind::Audio(0), OutBind::Control(0), OutBind::Audio(1)]);
        assert_eq!(l.out_ports[0].channels, Some(Channels::Any(deck_catalog::AnyChannels::Any)));
        assert_eq!(l.out_ports[2].channels, Some(Channels::Count(2)), "a cv port is two channels");
        assert_eq!(l.targets, [(1, "fx.gain".to_string(), deck_catalog::Mode::Mul)]);
        assert_eq!(l.events_in, [EventPort { name: "n".into(), names: vec!["music.note".into()], props: vec!["pitch".into()], capacity: 16 }]);
        assert!(l.events_out.is_empty());
        assert_eq!(l.domain, Domain::Audio);
    }
}
