//! The module host (#280, #305): runs rig modules – wasm built against deck-module's SDK – behind a [`Backend`].
//! - [`Loaded::load`] reads a module's descriptor from the `deck.module` section without running anything, checks it and
//!   the backend's imports and exports, and compiles it once (the backend caches).
//! - [`Instance`] is one running module: the ABI's `Io` written and read through the layout deck-module computes from
//!   the same type. A trap (a panic is one) or a time-budget overrun *silences that instance*: it is logged once, its
//!   outputs stay silent, and the show goes on.
//! - The [`Backend`] is the only part that touches a wasm engine: `compile`, `instantiate`, `call`, `write`, `read`,
//!   `set_budget`. The native deck uses [`wasmtime_backend::Wasmtime`]; the conformance suite (tests/) is generic over it.
use anyhow::{anyhow, bail, Result};
use deck_core::tempo::TempoMap;
use deck_module::abi::{get_str, put_str, EventRec, ABI_VERSION, MAX_AUDIO_PORTS, MAX_BLOCK, MAX_CONTROL_IN, MAX_EVENTS, MAX_VARS, STR_LEN};
use deck_module::host::{read_descriptor, Descriptor, Dir, Offsets, PortKind};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "wasmtime")]
pub mod wasmtime_backend;

/// a module's linear memory is capped here (the SDK's `Io` is ~130 KB; this leaves room for a creator's tables)
pub const MEMORY_LIMIT: usize = 64 << 20;
/// log lines a module may print: a burst, then this many a second (the rest is counted, once reported)
const LOG_BURST: f64 = 10.0;
const LOG_PER_SECOND: f64 = 10.0;
/// the one import a module may have
pub const LOG_IMPORT: (&str, &str) = ("env", "deck_log");
/// the exports every module has
pub const REQUIRED_EXPORTS: [&str; 7] = ["memory", "dm_abi", "dm_io", "dm_new", "dm_set_param", "dm_process", "dm_control"];

/// the module's entry points the host calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Export {
    /// `dm_abi() -> u32`
    Abi,
    /// `dm_io() -> u32`: where the `Io` lies in the module's memory
    Io,
    /// `dm_new()`: builds the module (runs its `init`)
    New,
    /// `dm_set_param(index: u32, value: f32)`
    SetParam,
    /// `dm_process(frames: u32)`
    Process,
    /// `dm_control(frames: u32)`
    Control,
}

impl Export {
    pub const ALL: [Export; 6] = [Export::Abi, Export::Io, Export::New, Export::SetParam, Export::Process, Export::Control];

    pub fn name(self) -> &'static str {
        match self {
            Export::Abi => "dm_abi",
            Export::Io => "dm_io",
            Export::New => "dm_new",
            Export::SetParam => "dm_set_param",
            Export::Process => "dm_process",
            Export::Control => "dm_control",
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
    /// Calls an export; the result of those that return a value (`Abi`, `Io`), else 0.
    fn call(&mut self, export: Export, args: &[Arg]) -> Result<u32, Fault>;
    /// the memory's size in bytes now
    fn memory_size(&self) -> usize;
    /// writes `bytes` at the absolute memory offset `off` (in range: the host checked the `Io` lies in the memory)
    fn write(&mut self, off: usize, bytes: &[u8]);
    fn read(&self, off: usize, out: &mut [u8]);
    /// the time the next `call` may take, in milliseconds
    fn set_budget(&mut self, ms: u64);
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

/// How a module's descriptor maps onto graph ports: inputs are the declared audio / control inputs in declaration order,
/// then the parameters (every parameter is also a control input); outputs are the declared audio outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub inputs: Vec<InBind>,
    /// the module's audio output port behind each graph output
    pub outputs: Vec<usize>,
}

impl Layout {
    /// Err when the module needs something this host cannot do yet (video, control outputs)
    pub fn of(d: &Descriptor) -> Result<Layout> {
        let (mut audio, mut control) = (0, 0);
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut audio_out = 0;
        for p in &d.ports {
            match (p.dir, p.kind) {
                (_, PortKind::Video) => bail!("module '{}': video port '{}' – a video module is not supported by this host yet (#185)", d.name, p.name),
                (Dir::Out, PortKind::Control) => {
                    bail!("module '{}': control output '{}' – control outputs of modules are not wired into the graph yet", d.name, p.name)
                }
                (Dir::In, PortKind::Audio) => {
                    inputs.push(InBind::Audio(audio));
                    audio += 1;
                }
                (Dir::In, PortKind::Control) => {
                    inputs.push(InBind::Control(control));
                    control += 1;
                }
                (Dir::Out, PortKind::Audio) => {
                    outputs.push(audio_out);
                    audio_out += 1;
                }
                (_, PortKind::Event) => {}
            }
        }
        inputs.extend((0..d.params.len()).map(InBind::Param));
        Ok(Layout { inputs, outputs })
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
    compiled: B::Compiled,
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
        let desc = Descriptor::parse(json).map_err(|e| anyhow!(e))?;
        let layout = Layout::of(&desc)?;
        let (compiled, from_disk, compile) = B::compile(engine, wasm, &hash)?;
        for (m, n) in B::imports(&compiled) {
            if (m.as_str(), n.as_str()) != LOG_IMPORT {
                bail!("module '{}' imports {m}.{n} – a module may import only env.deck_log", desc.name);
            }
        }
        for export in REQUIRED_EXPORTS {
            if !B::has_export(&compiled, export) {
                bail!("module '{}' does not export `{export}`", desc.name);
            }
        }
        Ok(Loaded { desc, layout, hash, compile, from_disk, compiled })
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

/// One running module.
pub struct Instance<B: Backend> {
    loaded: Arc<Loaded<B>>,
    backend: B,
    io: usize,
    off: Offsets,
    rate: f64,
    silenced: Option<String>,
    /// µs of the last `process` / the slowest so far
    last_us: u32,
    max_us: u32,
}

impl<B: Backend> Instance<B> {
    /// Instantiates `loaded` for `rate` Hz with `seed` (the same seed plays the same on every deck). The module is built
    /// with its parameters at their defaults; tempo 120 bpm, 4 a bar, from frame 0 until `set_tempo`.
    pub fn new(loaded: &Arc<Loaded<B>>, rate: f64, seed: u64) -> Result<Instance<B>> {
        let name = &loaded.desc.name;
        let mut backend = B::instantiate(&loaded.compiled, name)?;
        backend.set_budget(1000);
        let abi = backend.call(Export::Abi, &[]).map_err(|f| anyhow!("{name}: dm_abi: {}", fault_text(&f)))?;
        if abi != ABI_VERSION {
            bail!("module '{name}' speaks ABI {abi}, this host speaks {ABI_VERSION}");
        }
        let io = backend.call(Export::Io, &[]).map_err(|f| anyhow!("{name}: dm_io: {}", fault_text(&f)))? as usize;
        let off = Offsets::get();
        match io.checked_add(off.size) {
            Some(end) if end <= backend.memory_size() => {}
            _ => bail!("module '{name}': its Io does not lie in its memory"),
        }
        let mut me = Instance { loaded: loaded.clone(), backend, io, off, rate, silenced: None, last_us: 0, max_us: 0 };
        me.put_f64(me.off.rate, rate);
        me.put_u64(me.off.seed, seed);
        me.set_tempo(&TempoMap::new(120.0, 4, rate));
        // building the module runs its `init`: it gets the same budget as a block
        me.backend.set_budget(50);
        me.backend.call(Export::New, &[]).map_err(|f| anyhow!("{}: dm_new: {}", me.loaded.desc.name, fault_text(&f)))?;
        Ok(me)
    }

    pub fn descriptor(&self) -> &Descriptor {
        &self.loaded.desc
    }

    pub fn layout(&self) -> &Layout {
        &self.loaded.layout
    }

    /// the backend (tests, meters)
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// why this instance is silent (a trap, a budget overrun), if it is
    pub fn silenced(&self) -> Option<&str> {
        self.silenced.as_deref()
    }

    /// microseconds the last `process` took, and the slowest one
    pub fn cpu_us(&self) -> (u32, u32) {
        (self.last_us, self.max_us)
    }

    fn put(&mut self, at: usize, bytes: &[u8]) {
        let io = self.io;
        self.backend.write(io + at, bytes);
    }

    fn put_f64(&mut self, at: usize, v: f64) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_u64(&mut self, at: usize, v: u64) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_f32(&mut self, at: usize, v: f32) {
        self.put(at, &v.to_le_bytes());
    }

    fn get_u32(&self, at: usize) -> u32 {
        let mut b = [0u8; 4];
        self.backend.read(self.io + at, &mut b);
        u32::from_le_bytes(b)
    }

    pub fn set_tempo(&mut self, m: &TempoMap) {
        let o = self.off;
        self.put_f64(o.tempo_frame, m.frame);
        self.put_f64(o.tempo_beat, m.beat);
        self.put_f64(o.bpm, m.bpm);
        self.put_f64(o.rate, m.rate);
        self.put_u32(o.running, u32::from(m.running));
        self.put_u32(o.per_bar, m.per_bar);
    }

    /// the character the module plays for (empty = none)
    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        let o = self.off;
        for (at, s) in [(o.character_id, id), (o.character_persona, persona), (o.character_mood, mood)] {
            let mut buf = [0u8; STR_LEN];
            put_str(&mut buf, s);
            self.put(at, &buf);
        }
    }

    /// scene and world variables, key → number (at most `MAX_VARS`)
    pub fn set_vars(&mut self, vars: &[(String, f32)]) {
        let o = self.off;
        let n = vars.len().min(MAX_VARS);
        for (i, (k, v)) in vars.iter().take(n).enumerate() {
            let mut key = [0u8; deck_module::abi::VAR_KEY_LEN];
            put_str(&mut key, k);
            self.put(o.var_keys + i * deck_module::abi::VAR_KEY_LEN, &key);
            self.put_f32(o.var_values + i * 4, *v);
        }
        self.put_u32(o.vars_len, n as u32);
    }

    /// parameter `index`, clamped by the module; a silenced instance ignores it
    pub fn set_param(&mut self, index: usize, value: f32) {
        if self.silenced.is_some() {
            return;
        }
        // a deadline is relative to when it is set: the last call's would be long past
        self.backend.set_budget(50);
        let r = self.backend.call(Export::SetParam, &[Arg::U32(index as u32), Arg::F32(value)]);
        self.check(r, "dm_set_param");
    }

    /// the n-th declared control input's value for the next call
    pub fn set_control_in(&mut self, port: usize, value: f32) {
        if port < MAX_CONTROL_IN {
            let at = self.off.control_in + port * 4;
            self.put_f32(at, value);
        }
    }

    /// the n-th declared audio input for the next call
    pub fn set_audio_in(&mut self, port: usize, l: &[f32], r: &[f32]) {
        if port >= MAX_AUDIO_PORTS {
            return;
        }
        let n = l.len().min(r.len()).min(MAX_BLOCK);
        let base = self.off.audio_in + port * 2 * MAX_BLOCK * 4;
        for (ch, src) in [l, r].into_iter().enumerate() {
            self.put(base + ch * MAX_BLOCK * 4, bytemuck::cast_slice(&src[..n]));
        }
    }

    /// the audio output `port` of the last `process` into `l` / `r` (their lengths = the frames asked)
    pub fn read_audio_out(&mut self, port: usize, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(MAX_BLOCK);
        if port >= MAX_AUDIO_PORTS {
            l.fill(0.0);
            r.fill(0.0);
            return;
        }
        let base = self.io + self.off.audio_out + port * 2 * MAX_BLOCK * 4;
        for (ch, dst) in [l, r].into_iter().enumerate() {
            self.backend.read(base + ch * MAX_BLOCK * 4, bytemuck::cast_slice_mut(&mut dst[..n]));
        }
    }

    fn budget(&self, frames: u32) -> u64 {
        ((0.75 * f64::from(frames) / self.rate * 1000.0).ceil() as u64).clamp(2, 50)
    }

    /// One audio call of `frames` (≤ MAX_BLOCK) frames starting at `frame`. False when the instance is (now) silent.
    pub fn process(&mut self, frame: f64, frames: u32) -> bool {
        self.call(frame, frames, false)
    }

    /// One control tick
    pub fn control(&mut self, frame: f64, frames: u32) -> bool {
        self.call(frame, frames, true)
    }

    fn call(&mut self, frame: f64, frames: u32, control: bool) -> bool {
        if self.silenced.is_some() {
            return false;
        }
        let frames = frames.min(MAX_BLOCK as u32);
        self.put_f64(self.off.frame, frame);
        let ms = self.budget(frames);
        self.backend.set_budget(ms);
        let t = self.backend.clock_us();
        let export = if control { Export::Control } else { Export::Process };
        let r = self.backend.call(export, &[Arg::U32(frames)]);
        let us = self.backend.clock_us().saturating_sub(t).min(u64::from(u32::MAX)) as u32;
        (self.last_us, self.max_us) = (us, self.max_us.max(us));
        self.check(r, export.name())
    }

    /// a trap silences the instance: logged once, outputs stay silent
    fn check(&mut self, r: Result<u32, Fault>, what: &str) -> bool {
        match r {
            Ok(_) => true,
            Err(f) => {
                let msg = match f {
                    Fault::Budget => format!("{what} ran past its time budget"),
                    Fault::Trap(why) => format!("{what}: {why}"),
                };
                tracing::error!("module '{}' silenced – {msg}", self.loaded.desc.name);
                self.silenced = Some(msg);
                false
            }
        }
    }

    /// the events the last call emitted (allocates – for tests and non-realtime consumers)
    pub fn events(&mut self) -> Vec<EventRec> {
        let n = (self.get_u32(self.off.events_len) as usize).min(MAX_EVENTS);
        let size = self.off.event_size;
        let mut buf = vec![0u8; MAX_EVENTS * size];
        self.backend.read(self.io + self.off.events, &mut buf[..n * size]);
        (0..n).filter_map(|i| deck_module::host::event_at(&buf, i)).collect()
    }

    pub fn events_dropped(&mut self) -> u32 {
        self.get_u32(self.off.events_dropped)
    }

    /// a character's id as the module reads it (tests)
    pub fn character_id(&mut self) -> String {
        let mut buf = [0u8; STR_LEN];
        self.backend.read(self.io + self.off.character_id, &mut buf);
        get_str(&buf).to_owned()
    }
}

fn fault_text(f: &Fault) -> String {
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
        let d = |ports: &str| {
            Descriptor::parse(&format!(
                "{{\"abi\":1,\"name\":\"m\",\"version\":\"1\",\"params\":[{{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":0}}],\"ports\":[{ports}]}}"
            ))
            .unwrap()
        };
        let ok = d("{\"name\":\"x\",\"dir\":\"in\",\"kind\":\"audio\"},{\"name\":\"c\",\"dir\":\"in\",\"kind\":\"control\"},{\"name\":\"o\",\"dir\":\"out\",\"kind\":\"audio\"},{\"name\":\"e\",\"dir\":\"out\",\"kind\":\"event\"}");
        let l = Layout::of(&ok).unwrap();
        assert_eq!(l.inputs, [InBind::Audio(0), InBind::Control(0), InBind::Param(0)]);
        assert_eq!(l.outputs, [0]);
        assert!(Layout::of(&d("{\"name\":\"v\",\"dir\":\"out\",\"kind\":\"video\"}")).unwrap_err().to_string().contains("video"));
        assert!(Layout::of(&d("{\"name\":\"v\",\"dir\":\"out\",\"kind\":\"control\"}")).unwrap_err().to_string().contains("control output"));
    }
}
