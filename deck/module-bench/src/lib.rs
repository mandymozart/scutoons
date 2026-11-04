//! module-bench – the module runtime benchmark (#306, agents/research/MODULE-BENCH.md). One suite of workloads, generic
//! over deck-module-host's [`Backend`] – the host's own io staging, layout check, strict / show modes and restart run
//! unchanged on every runtime, so what differs between two rows of the table is the wasm engine:
//! - `fm16` (sixteen FM voices, this crate's modules/fm16) in calls of 128 frames, and 8 calls a 1024-frame block;
//! - `walk` (deck/modules/walk) with a full spread, 4 steps a beat;
//! - `analog` × 2 (deck/modules/analog, 8 voices each held: 16 voices at once);
//! - the runaway (`stress` mode 1, `loop {}`): the time from the budget to the stop, and a show restart's rebuild;
//! - the growth (`stress` mode 2, `memory.grow` of 1 MB a call after the host froze the memory).
//!
//! Per workload: compile and instantiate time, p50 / p99 / max / mean µs per sample (a call of 128 frames, or a
//! 1024-frame block), the fuel / gas units per call where the runtime counts them, a hash of every output sample (bit
//! equality across runtimes), the module's peak linear memory. [`report`] turns results into the table and the gates.
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_module::{Ev, QUANTUM};
use deck_module_host::{Backend, Instance, Loaded, Mode, Setup};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod report;
#[cfg(feature = "wasmi")]
pub mod wasmi_backend;

pub const RATE: f64 = 48_000.0;
/// one call of `QUANTUM` frames at 48 kHz, µs
pub const BUDGET_US: f64 = QUANTUM as f64 * 1e6 / RATE;

/// a monotonic clock in µs: `Instant` natively, the platform's `performance.now()` in a worker (the `bench.now_us` import)
#[cfg(not(target_arch = "wasm32"))]
pub fn now_us() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1e6
}

#[cfg(target_arch = "wasm32")]
pub fn now_us() -> f64 {
    #[link(wasm_import_module = "bench")]
    extern "C" {
        fn now_us() -> f64;
    }
    // SAFETY: a plain import without arguments (www/bench.ts)
    unsafe { now_us() }
}

/// What the suite needs of a backend beyond the host's [`Backend`]: the deterministic units (fuel / gas) of the last
/// call, and the units a millisecond (from the fm16 run), so the budget of a fuel / gas runtime is a time.
pub trait Runtime: Backend {
    fn units(&self) -> Option<u64> {
        None
    }
    fn calibrate(_engine: &Self::Engine, _units_per_ms: f64) {}
}

#[cfg(feature = "wasmtime")]
impl Runtime for deck_module_host::wasmtime_backend::Wasmtime {}

/// the workloads' wasm
#[derive(Default, Clone)]
pub struct Modules {
    pub fm16: Vec<u8>,
    pub walk: Vec<u8>,
    pub analog: Vec<u8>,
    pub stress: Vec<u8>,
}

impl Modules {
    pub const NAMES: [&'static str; 4] = ["fm16", "walk", "analog", "stress"];

    pub fn set(&mut self, name: &str, bytes: Vec<u8>) {
        match name {
            "fm16" => self.fm16 = bytes,
            "walk" => self.walk = bytes,
            "analog" => self.analog = bytes,
            _ => self.stress = bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// calls of 128 frames per workload (aggregated: a browser's clock is 5–100 µs coarse)
    pub blocks: usize,
    /// 1024-frame blocks of fm16 (8 calls each)
    pub blocks_1024: usize,
    /// whether this runtime can stop a runaway itself (a budget); the plain browser runtime cannot – its watchdog is the page's
    pub runaway: bool,
    pub grow: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            blocks: 2000,
            blocks_1024: 250,
            runaway: true,
            grow: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Workload {
    pub name: String,
    /// frames per sample: 128 (one call) or 1024 (8 calls)
    pub frames: u32,
    pub samples: usize,
    pub compile_us: f64,
    pub instantiate_us: f64,
    pub first_us: f64,
    pub p50_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub mean_us: f64,
    /// the sample's real-time budget (frames at 48 kHz), µs
    pub budget_us: f64,
    pub p99_pct: f64,
    /// fuel / gas per sample (deterministic), if the runtime counts
    pub units: Option<f64>,
    /// FNV-1a 64 of every output sample's bits
    pub hash: String,
    pub memory_bytes: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Runaway {
    /// how the stop works: epoch, fuel, gas, watchdog
    pub how: String,
    pub budget_ms: f64,
    /// from the call to its stop
    pub stopped_after_us: Option<f64>,
    /// past the budget
    pub over_budget_us: Option<f64>,
    /// a show's restart: the rebuild from the cached compile (or the watchdog's new worker)
    pub restart_us: Option<f64>,
    pub fault: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Grow {
    /// `memory.grow` per call: the old size in pages, -1 refused
    pub results: Vec<f32>,
    pub memory_before: u64,
    pub memory_after: u64,
    pub us_per_call: f64,
    /// what the host did (strict: a trap silences)
    pub fault: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeResult {
    /// wasmtime · wasmi · wasmtime-gas (native) · web · web-mvp · web-gas · web-wasmi (a worker)
    pub runtime: String,
    pub place: String,
    pub workloads: Vec<Workload>,
    pub runaway: Option<Runaway>,
    pub grow: Option<Grow>,
    /// the clock's smallest step seen, µs
    pub timer_us: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Size {
    pub name: String,
    pub bytes: u64,
    pub gz: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Device {
    /// pc · x1 · s22 · … (the results file's name)
    pub label: String,
    pub os: String,
    pub cpu: String,
    pub user_agent: String,
    pub when: String,
    pub cores: u32,
    /// the page's notes: cross-origin isolated, memory
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Results {
    pub device: Device,
    pub runtimes: Vec<RuntimeResult>,
    pub sizes: Vec<Size>,
}

/// FNV-1a 64
#[derive(Clone, Copy)]
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn f32s(&mut self, v: &[f32]) {
        for x in v {
            for b in x.to_bits().to_le_bytes() {
                self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

/// the clock's smallest nonzero step (a browser rounds `performance.now()`)
pub fn timer_resolution_us() -> f64 {
    let mut best = f64::INFINITY;
    let mut last = now_us();
    let mut seen = 0;
    while seen < 50 {
        let t = now_us();
        if t > last {
            best = best.min(t - last);
            seen += 1;
            last = t;
        }
    }
    best
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// samples (µs each) → the workload's numbers
fn stats(w: &mut Workload, samples: &[f64], units: &[u64]) {
    let mut s = samples.to_vec();
    w.first_us = s.first().copied().unwrap_or(0.0);
    w.samples = s.len();
    s.sort_by(|a, b| a.total_cmp(b));
    w.p50_us = pct(&s, 0.5);
    w.p99_us = pct(&s, 0.99);
    w.max_us = s.last().copied().unwrap_or(0.0);
    w.mean_us = s.iter().sum::<f64>() / s.len().max(1) as f64;
    w.budget_us = BUDGET_US * f64::from(w.frames) / QUANTUM as f64;
    w.p99_pct = 100.0 * w.p99_us / w.budget_us;
    if !units.is_empty() {
        w.units = Some(units.iter().sum::<u64>() as f64 / units.len() as f64);
    }
}

fn load<B: Runtime>(engine: &B::Engine, wasm: &[u8]) -> Result<Arc<Loaded<B>>> {
    Loaded::<B>::load(engine, wasm).map(Arc::new)
}

/// a new instance, timed; strict and unbudgeted (a slow interpreter is measured, not trapped)
fn instance<B: Runtime>(loaded: &Arc<Loaded<B>>, setup: Setup) -> Result<(Instance<B>, f64)> {
    let t = now_us();
    let i = Instance::new(loaded, setup)?;
    Ok((i, now_us() - t))
}

fn param<B: Runtime>(i: &mut Instance<B>, name: &str, v: f32) -> Result<()> {
    let k = i
        .loaded()
        .param_index(name)
        .ok_or_else(|| anyhow!("no parameter '{name}'"))?;
    i.set_param(k, v);
    Ok(())
}

fn call<B: Runtime>(i: &mut Instance<B>, k: u64) -> Result<()> {
    if i.run(k as f64 * QUANTUM as f64, QUANTUM as u32) {
        Ok(())
    } else {
        Err(anyhow!(
            "call {k} did not run: {}",
            i.silenced().unwrap_or("?")
        ))
    }
}

fn hash_out<B: Runtime>(i: &Instance<B>, h: &mut Fnv) {
    let out = i.audio_out(0);
    for c in 0..out.ch() {
        h.f32s(out.chan(c));
    }
}

fn fm16<B: Runtime>(
    engine: &B::Engine,
    wasm: &[u8],
    frames: u32,
    n: usize,
    calibrate: bool,
) -> Result<Workload> {
    let mut w = Workload {
        name: "fm16".into(),
        frames,
        ..Workload::default()
    };
    let loaded = load::<B>(engine, wasm)?;
    w.compile_us = loaded.compile.as_secs_f64() * 1e6;
    let (mut i, inst) = instance(&loaded, Setup::new(RATE, 1).unbudgeted())?;
    w.instantiate_us = inst;
    param(&mut i, "index", 3.0)?;
    let calls = frames as u64 / QUANTUM as u64;
    let (mut samples, mut units) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let mut h = Fnv::new();
    let mut k = 0u64;
    for _ in 0..n {
        let t = now_us();
        let mut u = 0;
        for _ in 0..calls {
            call(&mut i, k)?;
            k += 1;
            u += i.backend().and_then(|b| b.units()).unwrap_or(0);
            hash_out(&i, &mut h);
        }
        samples.push(now_us() - t);
        if i.backend().and_then(|b| b.units()).is_some() {
            units.push(u);
        }
        w.memory_bytes = w.memory_bytes.max(i.memory_size() as u64);
    }
    // the hash covers every call's output; the clock covers the hashing too (a few µs, alike on every runtime)
    w.hash = h.hex();
    stats(&mut w, &samples, &units);
    if calibrate {
        if let Some(u) = w.units {
            B::calibrate(engine, u / (w.mean_us / 1000.0).max(1e-9));
        }
    }
    Ok(w)
}

fn walk<B: Runtime>(engine: &B::Engine, wasm: &[u8], n: usize) -> Result<Workload> {
    let mut w = Workload {
        name: "walk".into(),
        frames: QUANTUM as u32,
        ..Workload::default()
    };
    let loaded = load::<B>(engine, wasm)?;
    w.compile_us = loaded.compile.as_secs_f64() * 1e6;
    let (mut i, inst) = instance(&loaded, Setup::new(RATE, 7).unbudgeted())?;
    w.instantiate_us = inst;
    param(&mut i, "spread", 1.0)?;
    param(&mut i, "rate", 4.0)?;
    i.set_tempo(&TempoMap::new(132.0, 4, RATE));
    let (mut samples, mut units) = (Vec::with_capacity(n), Vec::new());
    let mut h = Fnv::new();
    for k in 0..n as u64 {
        let t = now_us();
        call(&mut i, k)?;
        samples.push(now_us() - t);
        units.extend(i.backend().and_then(|b| b.units()));
        hash_out(&i, &mut h);
        for e in i.events_out(0) {
            h.f32s(&[e.pitch_hz(), e.frame as f32, e.id as f32]);
        }
        w.memory_bytes = w.memory_bytes.max(i.memory_size() as u64);
    }
    w.hash = h.hex();
    stats(&mut w, &samples, &units);
    Ok(w)
}

/// two `analog` instances, eight notes held on each: sixteen voices; a sample is both calls
fn analog<B: Runtime>(engine: &B::Engine, wasm: &[u8], n: usize) -> Result<Workload> {
    let mut w = Workload {
        name: "analog x2".into(),
        frames: QUANTUM as u32,
        ..Workload::default()
    };
    let loaded = load::<B>(engine, wasm)?;
    w.compile_us = loaded.compile.as_secs_f64() * 1e6;
    let mut synths = Vec::new();
    for s in 0..2u32 {
        let (i, inst) = instance(&loaded, Setup::new(RATE, 1 + u64::from(s)).unbudgeted())?;
        w.instantiate_us += inst / 2.0;
        synths.push(i);
    }
    let (mut samples, mut units) = (Vec::with_capacity(n), Vec::new());
    let mut h = Fnv::new();
    for k in 0..n as u64 {
        if k == 0 {
            for (s, i) in synths.iter_mut().enumerate() {
                let notes: Vec<Ev> = (0..8u32)
                    .map(|v| {
                        Ev::note_on(
                            0,
                            s as u32 * 8 + v,
                            110.0 * (1.0 + (s as f32 * 8.0 + v as f32) / 5.0),
                            f32::NAN,
                            0.8,
                        )
                    })
                    .collect();
                i.set_events_in(0, &notes);
            }
        }
        let t = now_us();
        let mut u = 0;
        for i in synths.iter_mut() {
            call(i, k)?;
            u += i.backend().and_then(|b| b.units()).unwrap_or(0);
        }
        samples.push(now_us() - t);
        if synths[0].backend().and_then(|b| b.units()).is_some() {
            units.push(u);
        }
        for i in &synths {
            hash_out(i, &mut h);
            w.memory_bytes = w.memory_bytes.max(i.memory_size() as u64);
        }
    }
    w.hash = h.hex();
    stats(&mut w, &samples, &units);
    Ok(w)
}

/// `how`: what stops it on this runtime. Strict first (the time from the deadline to the stop), then a show (the
/// restart's rebuild, on this thread – a deck runs it on its loader thread).
fn runaway<B: Runtime>(engine: &B::Engine, wasm: &[u8], how: &str) -> Result<Runaway> {
    let loaded = load::<B>(engine, wasm)?;
    let budget_ms = ((0.75 * BUDGET_US / 1000.0).ceil()).clamp(2.0, 50.0);
    let mut r = Runaway {
        how: how.into(),
        budget_ms,
        ..Runaway::default()
    };
    let (mut i, _) = instance(&loaded, Setup::new(RATE, 1))?;
    param(&mut i, "mode", 1.0)?;
    let t = now_us();
    let ran = i.run(0.0, QUANTUM as u32);
    let us = now_us() - t;
    if !ran {
        r.stopped_after_us = Some(us);
        r.over_budget_us = Some(us - budget_ms * 1000.0);
        r.fault = i.silenced().unwrap_or("").to_string();
    }
    let (mut s, _) = instance(&loaded, Setup::new(RATE, 1).mode(Mode::Show))?;
    param(&mut s, "mode", 1.0)?;
    s.run(0.0, QUANTUM as u32);
    // the first restart waits one quantum
    s.run(QUANTUM as f64, QUANTUM as u32);
    if let Some(job) = s.take_rebuild() {
        let t = now_us();
        let fresh = job.run();
        r.restart_us = Some(now_us() - t);
        s.offer(fresh);
    }
    Ok(r)
}

/// The runaway on a runtime that cannot stop it (the browser's WebAssembly without gas): the call never returns – the
/// page's watchdog terminates the worker and times a new one (www/bench.ts).
/// `mark` runs right before the call (the watchdog's clock starts there).
pub fn hang<B: Runtime>(engine: &B::Engine, wasm: &[u8], mark: impl Fn()) -> Result<()> {
    let loaded = load::<B>(engine, wasm)?;
    let (mut i, _) = instance(&loaded, Setup::new(RATE, 1))?;
    param(&mut i, "mode", 1.0)?;
    mark();
    i.run(0.0, QUANTUM as u32);
    Err(anyhow!("the runaway returned"))
}

fn grow<B: Runtime>(engine: &B::Engine, wasm: &[u8]) -> Result<Grow> {
    let loaded = load::<B>(engine, wasm)?;
    let (mut i, _) = instance(&loaded, Setup::new(RATE, 1).unbudgeted())?;
    param(&mut i, "mode", 2.0)?;
    let mut g = Grow {
        memory_before: i.memory_size() as u64,
        ..Grow::default()
    };
    let t = now_us();
    let mut calls = 0;
    for k in 0..8u64 {
        calls += 1;
        if !i.run(k as f64 * QUANTUM as f64, QUANTUM as u32) {
            g.fault = Some(i.silenced().unwrap_or("did not run").to_string());
            break;
        }
        g.results.push(i.control_out(0).0);
    }
    g.us_per_call = (now_us() - t) / f64::from(calls);
    g.memory_after = i.memory_size() as u64;
    Ok(g)
}

fn guard(r: Result<Workload>, name: &str, frames: u32) -> Workload {
    match r {
        Ok(w) => w,
        Err(e) => Workload {
            name: name.into(),
            frames,
            error: Some(format!("{e:#}")),
            ..Workload::default()
        },
    }
}

/// The whole suite on one runtime. `how`: what stops a runaway here (`None`: nothing in-thread – skipped).
pub fn run_suite<B: Runtime>(
    engine: &B::Engine,
    runtime: &str,
    place: &str,
    m: &Modules,
    cfg: &Config,
    how: Option<&str>,
) -> RuntimeResult {
    let mut r = RuntimeResult {
        runtime: runtime.into(),
        place: place.into(),
        timer_us: timer_resolution_us(),
        ..RuntimeResult::default()
    };
    let q = QUANTUM as u32;
    r.workloads.push(guard(
        fm16::<B>(engine, &m.fm16, q, cfg.blocks, true),
        "fm16",
        q,
    ));
    r.workloads.push(guard(
        fm16::<B>(engine, &m.fm16, 8 * q, cfg.blocks_1024, false),
        "fm16",
        8 * q,
    ));
    r.workloads
        .push(guard(walk::<B>(engine, &m.walk, cfg.blocks), "walk", q));
    r.workloads.push(guard(
        analog::<B>(engine, &m.analog, cfg.blocks),
        "analog x2",
        q,
    ));
    if let (true, Some(how)) = (cfg.runaway, how) {
        match runaway::<B>(engine, &m.stress, how) {
            Ok(x) => r.runaway = Some(x),
            Err(e) => r.error = Some(format!("runaway: {e:#}")),
        }
    }
    if cfg.grow {
        match grow::<B>(engine, &m.stress) {
            Ok(x) => r.grow = Some(x),
            Err(e) => r.error = Some(format!("grow: {e:#}")),
        }
    }
    r
}
