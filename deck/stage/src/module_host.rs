//! The module host (#280, feature `modules`, research/RIG-EDITING.md §4): runs rig modules – wasm built against
//! deck-module's SDK – in wasmtime.
//! - `Library` loads a `.wasm`: its descriptor is read from the `deck.module` section without running anything, checked,
//!   the code compiled once (cached in memory and, with `DECK_MODULE_CACHE`, on disk, keyed by the wasm's SHA-256) and
//!   its ports registered as the node kind `module:<name>` (deck-space's modreg.rs). `autoload` loads every `*.wasm` of
//!   `DECK_MODULES` before the stage builds a scene.
//! - `Instance` is one running module: its own store and memory, the ABI's `Io` written and read through the layout
//!   deck-module computes from the same type. Sandboxed – the module imports one function (`env.deck_log`, rate-limited
//!   per module), nothing else; memory is capped. A trap (a panic is one) or a time-budget overrun (wasmtime's epoch
//!   interruption, a 1 ms ticker) *silences that instance*: it is logged once, its outputs stay silent, and the show goes
//!   on. Member code is never run on the server – this runs on a deck, in the browser's engine, or in a test.
use anyhow::{anyhow, bail, Context, Result};
use deck_core::tempo::TempoMap;
use deck_module::abi::{get_str, put_str, EventRec, ABI_VERSION, MAX_AUDIO_PORTS, MAX_BLOCK, MAX_CONTROL_IN, MAX_EVENTS, MAX_VARS, STR_LEN};
use deck_module::host::{read_descriptor, Descriptor, Dir, Offsets, PortKind};
use deck_space::graph::PortType;
use deck_space::modreg::{self, ModuleKind};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use wasmtime::{Caller, Config, Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

/// a module's linear memory is capped here (the SDK's `Io` is ~130 KB; this leaves room for a creator's tables)
pub const MEMORY_LIMIT: usize = 64 << 20;
/// log lines a module may print: a burst, then this many a second (the rest is counted, once reported)
const LOG_BURST: f64 = 10.0;
const LOG_PER_SECOND: f64 = 10.0;

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

/// A loaded module: checked, compiled, registered as a node kind. Cheap to share.
pub struct Loaded {
    pub desc: Descriptor,
    pub layout: Layout,
    pub kind: ModuleKind,
    /// hex SHA-256 of the wasm
    pub hash: String,
    pub compile: Duration,
    pub from_disk: bool,
    module: Module,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Loaded({} {} {})", self.desc.name, self.desc.version, &self.hash[..12])
    }
}

pub struct Library {
    engine: Engine,
    by_hash: Mutex<HashMap<String, Arc<Loaded>>>,
    by_name: Mutex<HashMap<String, Arc<Loaded>>>,
    cache: Option<PathBuf>,
}

static GLOBAL: OnceLock<Library> = OnceLock::new();

impl Library {
    /// the process's library; its engine's epoch ticker (1 ms) starts with it
    pub fn global() -> &'static Library {
        GLOBAL.get_or_init(|| Library::new(std::env::var_os("DECK_MODULE_CACHE").map(PathBuf::from)))
    }

    /// a library of its own with a compile cache in `dir` (tests; the process's is `global`)
    pub fn with_cache(dir: &Path) -> Library {
        Library::new(Some(dir.to_owned()))
    }

    fn new(cache: Option<PathBuf>) -> Library {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).expect("wasmtime engine");
        let ticker = engine.clone();
        std::thread::Builder::new()
            .name("module-epoch".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(1));
                ticker.increment_epoch();
            })
            .expect("epoch ticker");
        Library { engine, by_hash: Mutex::new(HashMap::new()), by_name: Mutex::new(HashMap::new()), cache }
    }

    /// Loads a module from its wasm bytes: descriptor first (nothing runs), then the compile (cached), then the kind.
    pub fn load(&self, wasm: &[u8]) -> Result<Arc<Loaded>> {
        let hash = hex(&Sha256::digest(wasm));
        if let Some(l) = self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).get(&hash) {
            return Ok(l.clone());
        }
        let json = read_descriptor(wasm).ok_or_else(|| anyhow!("no `deck.module` section – the wasm was not built with deck-module"))?;
        let desc = Descriptor::parse(json).map_err(|e| anyhow!(e))?;
        let layout = Layout::of(&desc)?;
        let t = Instant::now();
        let (module, from_disk) = self.compile(&hash, wasm)?;
        let compile = t.elapsed();
        for imp in module.imports() {
            if (imp.module(), imp.name()) != ("env", "deck_log") {
                bail!("module '{}' imports {}.{} – a module may import only env.deck_log", desc.name, imp.module(), imp.name());
            }
        }
        for export in ["memory", "dm_abi", "dm_io", "dm_new", "dm_set_param", "dm_process", "dm_control"] {
            if module.get_export(export).is_none() {
                bail!("module '{}' does not export `{export}`", desc.name);
            }
        }
        // the graph's ports: control inputs and audio inputs as declared, the parameters as control inputs
        let ty = |k: &InBind| if matches!(k, InBind::Audio(_)) { PortType::Audio } else { PortType::Control };
        let port_names: Vec<&str> = desc
            .ports
            .iter()
            .filter(|p| p.dir == Dir::In && matches!(p.kind, PortKind::Audio | PortKind::Control))
            .map(|p| p.name.as_str())
            .chain(desc.params.iter().map(|p| p.name.as_str()))
            .collect();
        let inputs: Vec<(String, PortType)> = layout.inputs.iter().zip(&port_names).map(|(b, n)| ((*n).to_owned(), ty(b))).collect();
        let outputs: Vec<(String, PortType)> =
            desc.ports.iter().filter(|p| p.dir == Dir::Out && p.kind == PortKind::Audio).map(|p| (p.name.clone(), PortType::Audio)).collect();
        let defaults: Vec<(String, f32)> = desc.params.iter().map(|p| (p.name.clone(), p.default)).collect();
        let kind = modreg::register(&desc.name, &inputs, &outputs, &defaults);
        let loaded = Arc::new(Loaded { desc, layout, kind, hash: hash.clone(), compile, from_disk, module });
        self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).insert(hash, loaded.clone());
        self.by_name.lock().unwrap_or_else(|e| e.into_inner()).insert(loaded.desc.name.clone(), loaded.clone());
        Ok(loaded)
    }

    fn compile(&self, hash: &str, wasm: &[u8]) -> Result<(Module, bool)> {
        if let Some(dir) = &self.cache {
            let file = dir.join(format!("{hash}.cwasm"));
            if file.is_file() {
                // SAFETY: the file is our own `Module::serialize` output under a SHA-256 name in the deck's cache
                // directory; wasmtime also checks its version and compile settings and refuses a mismatch
                match unsafe { Module::deserialize_file(&self.engine, &file) } {
                    Ok(m) => return Ok((m, true)),
                    Err(e) => tracing::warn!("module cache {}: {e:#} – compiling again", file.display()),
                }
            }
        }
        let module = Module::new(&self.engine, wasm).context("compiling the module")?;
        if let Some(dir) = &self.cache {
            let write = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(dir.join(format!("{hash}.cwasm")), module.serialize().map_err(std::io::Error::other)?));
            if let Err(e) = write {
                tracing::warn!("module cache {}: {e}", dir.display());
            }
        }
        Ok((module, false))
    }

    /// every `*.wasm` of `dir`, in name order; an error in one is returned with its file, the others still load
    pub fn load_dir(&self, dir: &Path) -> Vec<(PathBuf, Result<Arc<Loaded>>)> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wasm")).collect())
            .unwrap_or_default();
        files.sort();
        files.into_iter().map(|f| (f.clone(), std::fs::read(&f).map_err(anyhow::Error::from).and_then(|b| self.load(&b)))).collect()
    }

    /// a loaded module by its descriptor name
    pub fn get(&self, name: &str) -> Option<Arc<Loaded>> {
        self.by_name.lock().unwrap_or_else(|e| e.into_inner()).get(name).cloned()
    }

    /// the loaded module behind a graph kind
    pub fn of_kind(&self, kind: ModuleKind) -> Option<Arc<Loaded>> {
        let want = kind.spec().kind.strip_prefix(modreg::PREFIX)?;
        self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).values().find(|l| l.desc.name == want && l.kind == kind).cloned()
    }
}

/// `DECK_MODULES` (a folder of `.wasm`) loaded once per process; errors logged, never fatal
pub fn autoload() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let Some(dir) = std::env::var_os("DECK_MODULES").map(PathBuf::from) else { return };
        for (file, r) in Library::global().load_dir(&dir) {
            match r {
                Ok(l) => tracing::info!(
                    "module '{}' {} ({} params) from {} – compiled in {:?}{}",
                    l.desc.name,
                    l.desc.version,
                    l.desc.params.len(),
                    file.display(),
                    l.compile,
                    if l.from_disk { " (cache)" } else { "" }
                ),
                Err(e) => tracing::error!("module {}: {e:#}", file.display()),
            }
        }
    });
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// a token bucket for `log!` (per module)
pub struct LogLimiter {
    tokens: f64,
    last: Option<Instant>,
    /// lines refused since the last one let through
    pub dropped: u64,
}

impl Default for LogLimiter {
    fn default() -> Self {
        LogLimiter { tokens: LOG_BURST, last: None, dropped: 0 }
    }
}

impl LogLimiter {
    /// whether a line at `now` may be printed; `dropped` counts the ones that may not
    pub fn allow(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last {
            self.tokens = (self.tokens + now.saturating_duration_since(last).as_secs_f64() * LOG_PER_SECOND).min(LOG_BURST);
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

struct State {
    name: String,
    limits: StoreLimits,
    log: LogLimiter,
}

/// One running module.
pub struct Instance {
    loaded: Arc<Loaded>,
    store: Store<State>,
    memory: Memory,
    io: usize,
    off: Offsets,
    f_set_param: TypedFunc<(u32, f32), ()>,
    f_process: TypedFunc<u32, ()>,
    f_control: TypedFunc<u32, ()>,
    rate: f64,
    silenced: Option<String>,
    /// µs of the last `process` / the slowest so far
    last_us: u32,
    max_us: u32,
}

impl Instance {
    /// Instantiates `loaded` for `rate` Hz with `seed` (the same seed plays the same on every deck). The module is built
    /// with its parameters at their defaults; tempo 120 bpm, 4 a bar, from frame 0 until `set_tempo`.
    pub fn new(loaded: &Arc<Loaded>, rate: f64, seed: u64) -> Result<Instance> {
        let lib = Library::global();
        let mut linker: Linker<State> = Linker::new(&lib.engine);
        linker.func_wrap("env", "deck_log", |mut c: Caller<'_, State>, level: u32, ptr: u32, len: u32| {
            let text = c
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .and_then(|m| m.data(&c).get(ptr as usize..(ptr as usize).saturating_add(len as usize)).map(|b| String::from_utf8_lossy(b).into_owned()));
            let Some(text) = text else { return };
            let dropped = c.data().log.dropped;
            if !c.data_mut().log.allow(Instant::now()) {
                return;
            }
            let name = c.data().name.clone();
            if dropped > 0 {
                tracing::warn!("module '{name}': {dropped} log lines dropped (rate limit)");
                c.data_mut().log.dropped = 0;
            }
            match level {
                0 => tracing::info!("module '{name}': {text}"),
                1 => tracing::warn!("module '{name}': {text}"),
                _ => tracing::error!("module '{name}': {text}"),
            }
        })?;
        let state = State {
            name: loaded.desc.name.clone(),
            limits: StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).instances(1).tables(10_000).memories(1).build(),
            log: LogLimiter::default(),
        };
        let mut store = Store::new(&lib.engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(1000);
        let instance = linker.instantiate(&mut store, &loaded.module).context("instantiating the module")?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("no memory export"))?;
        let abi = instance.get_typed_func::<(), u32>(&mut store, "dm_abi")?.call(&mut store, ())?;
        if abi != ABI_VERSION {
            bail!("module '{}' speaks ABI {abi}, this host speaks {ABI_VERSION}", loaded.desc.name);
        }
        let io = instance.get_typed_func::<(), u32>(&mut store, "dm_io")?.call(&mut store, ())? as usize;
        let off = Offsets::get();
        match io.checked_add(off.size) {
            Some(end) if end <= memory.data_size(&store) => {}
            _ => bail!("module '{}': its Io does not lie in its memory", loaded.desc.name),
        }
        let mut me = Instance {
            f_set_param: instance.get_typed_func(&mut store, "dm_set_param")?,
            f_process: instance.get_typed_func(&mut store, "dm_process")?,
            f_control: instance.get_typed_func(&mut store, "dm_control")?,
            loaded: loaded.clone(),
            store,
            memory,
            io,
            off,
            rate,
            silenced: None,
            last_us: 0,
            max_us: 0,
        };
        me.put_f64(me.off.rate, rate);
        me.put_u64(me.off.seed, seed);
        me.set_tempo(&TempoMap::new(120.0, 4, rate));
        // building the module runs its `init`: it gets the same budget as a block
        let new = instance.get_typed_func::<(), ()>(&mut me.store, "dm_new")?;
        me.store.set_epoch_deadline(50);
        new.call(&mut me.store, ()).map_err(|e| anyhow!("{}: dm_new: {}", me.loaded.desc.name, first_line(&e)))?;
        Ok(me)
    }

    pub fn descriptor(&self) -> &Descriptor {
        &self.loaded.desc
    }

    pub fn layout(&self) -> &Layout {
        &self.loaded.layout
    }

    /// why this instance is silent (a trap, a budget overrun), if it is
    pub fn silenced(&self) -> Option<&str> {
        self.silenced.as_deref()
    }

    /// microseconds the last `process` took, and the slowest one
    pub fn cpu_us(&self) -> (u32, u32) {
        (self.last_us, self.max_us)
    }

    fn data(&mut self) -> &mut [u8] {
        self.memory.data_mut(&mut self.store)
    }

    fn put_f64(&mut self, at: usize, v: f64) {
        let io = self.io;
        self.data()[io + at..io + at + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u64(&mut self, at: usize, v: u64) {
        let io = self.io;
        self.data()[io + at..io + at + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u32(&mut self, at: usize, v: u32) {
        let io = self.io;
        self.data()[io + at..io + at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_f32(&mut self, at: usize, v: f32) {
        let io = self.io;
        self.data()[io + at..io + at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn get_u32(&mut self, at: usize) -> u32 {
        let io = self.io;
        u32::from_le_bytes(self.data()[io + at..io + at + 4].try_into().expect("4 bytes"))
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
        let (o, io) = (self.off, self.io);
        for (at, s) in [(o.character_id, id), (o.character_persona, persona), (o.character_mood, mood)] {
            put_str(&mut self.data()[io + at..io + at + STR_LEN], s);
        }
    }

    /// scene and world variables, key → number (at most `MAX_VARS`)
    pub fn set_vars(&mut self, vars: &[(String, f32)]) {
        let (o, io) = (self.off, self.io);
        let n = vars.len().min(MAX_VARS);
        for (i, (k, v)) in vars.iter().take(n).enumerate() {
            let at = io + o.var_keys + i * deck_module::abi::VAR_KEY_LEN;
            put_str(&mut self.data()[at..at + deck_module::abi::VAR_KEY_LEN], k);
            self.put_f32(o.var_values + i * 4, *v);
        }
        self.put_u32(o.vars_len, n as u32);
    }

    /// parameter `index`, clamped by the module; a silenced instance ignores it
    pub fn set_param(&mut self, index: usize, value: f32) {
        if self.silenced.is_some() {
            return;
        }
        let r = self.f_set_param.call(&mut self.store, (index as u32, value));
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
        let (io, base) = (self.io, self.off.audio_in + port * 2 * MAX_BLOCK * 4);
        let data = self.data();
        for (ch, src) in [l, r].into_iter().enumerate() {
            let at = io + base + ch * MAX_BLOCK * 4;
            data[at..at + n * 4].copy_from_slice(f32_bytes(&src[..n]));
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
        let (io, base) = (self.io, self.off.audio_out + port * 2 * MAX_BLOCK * 4);
        let data = self.data();
        for (ch, dst) in [l, r].into_iter().enumerate() {
            let at = io + base + ch * MAX_BLOCK * 4;
            f32_bytes_mut(&mut dst[..n]).copy_from_slice(&data[at..at + n * 4]);
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
        let (ticks, t) = (self.budget(frames), Instant::now());
        self.store.set_epoch_deadline(ticks);
        let r = if control { self.f_control.call(&mut self.store, frames) } else { self.f_process.call(&mut self.store, frames) };
        let us = t.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
        (self.last_us, self.max_us) = (us, self.max_us.max(us));
        self.check(r, if control { "dm_control" } else { "dm_process" })
    }

    /// a trap silences the instance: logged once, outputs stay silent
    fn check(&mut self, r: Result<(), wasmtime::Error>, what: &str) -> bool {
        match r {
            Ok(()) => true,
            Err(e) => {
                let why = first_line(&e);
                let overrun = e.downcast_ref::<wasmtime::Trap>().is_some_and(|t| *t == wasmtime::Trap::Interrupt);
                let msg = if overrun { format!("{what} ran past its time budget") } else { format!("{what}: {why}") };
                tracing::error!("module '{}' silenced – {msg}", self.loaded.desc.name);
                self.silenced = Some(msg);
                false
            }
        }
    }

    /// the events the last call emitted (allocates – for tests and non-realtime consumers)
    pub fn events(&mut self) -> Vec<EventRec> {
        let n = (self.get_u32(self.off.events_len) as usize).min(MAX_EVENTS);
        let (io, at, size) = (self.io, self.off.events, self.off.event_size);
        let data = self.data();
        (0..n).filter_map(|i| deck_module::host::event_at(&data[io + at..io + at + MAX_EVENTS * size], i)).collect()
    }

    pub fn events_dropped(&mut self) -> u32 {
        self.get_u32(self.off.events_dropped)
    }

    /// a character's id as the module reads it (tests)
    pub fn character_id(&mut self) -> String {
        let (io, at) = (self.io, self.off.character_id);
        get_str(&self.data()[io + at..io + at + STR_LEN]).to_owned()
    }
}

fn first_line(e: &wasmtime::Error) -> String {
    format!("{e:#}").lines().next().unwrap_or("trap").to_owned()
}

fn f32_bytes(s: &[f32]) -> &[u8] {
    bytemuck::cast_slice(s)
}

fn f32_bytes_mut(s: &mut [f32]) -> &mut [u8] {
    bytemuck::cast_slice_mut(s)
}

/// set once any instance has been silenced (the HUD and the tests can look)
pub static ANY_SILENCED: AtomicBool = AtomicBool::new(false);

pub fn note_silenced() {
    ANY_SILENCED.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_limiter_lets_a_burst_through_then_a_steady_rate() {
        let mut l = LogLimiter::default();
        let t0 = Instant::now();
        let allowed = (0..50).filter(|_| l.allow(t0)).count();
        assert_eq!(allowed, 10, "the burst");
        assert_eq!(l.dropped, 40);
        // half a second later: five more
        let t1 = t0 + Duration::from_millis(500);
        assert_eq!((0..50).filter(|_| l.allow(t1)).count(), 5);
        // a long quiet time refills the burst, no more
        let t2 = t1 + Duration::from_secs(60);
        assert_eq!((0..50).filter(|_| l.allow(t2)).count(), 10);
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
