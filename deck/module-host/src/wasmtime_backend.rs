//! The wasmtime backend (what the native deck runs): the engine with its 1 ms epoch ticker – the time budget is
//! wasmtime's epoch interruption –, the compile cache (in memory by the caller, on disk here, keyed by the wasm's SHA-256)
//! and one store per instance with the memory capped at [`MEMORY_LIMIT`] (and frozen at its size by `freeze_memory`).
//! Determinism (ABI-V2 §6): NaNs canonicalized, relaxed SIMD off, threads and shared memory off (a module using them
//! fails to compile at load).
use crate::{log_line, Arg, Backend, Export, Fault, LogLimiter, MEMORY_LIMIT};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use wasmtime::{
    Caller, Config, Engine, Func, Linker, Memory, Module, ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder, TypedFunc, WasmParams, WasmResults,
};

/// wasmtime's engine, its ticker and the disk cache
pub struct WasmtimeEngine {
    engine: Engine,
    cache: Option<PathBuf>,
}

impl WasmtimeEngine {
    /// an engine of its own; its epoch ticker (1 ms) starts with it. `cache`: where compiled modules are kept.
    pub fn new(cache: Option<PathBuf>) -> WasmtimeEngine {
        let mut config = Config::new();
        config.epoch_interruption(true);
        // the same bits on every deck (ABI-V2 §6): one NaN, no relaxed SIMD (its results are the CPU's), no threads
        config.cranelift_nan_canonicalization(true);
        config.wasm_relaxed_simd(false);
        config.wasm_shared_everything_threads(false);
        let engine = Engine::new(&config).expect("wasmtime engine");
        let ticker = engine.clone();
        std::thread::Builder::new()
            .name("module-epoch".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(1));
                ticker.increment_epoch();
            })
            .expect("epoch ticker");
        WasmtimeEngine { engine, cache }
    }

    pub fn with_cache(dir: &Path) -> WasmtimeEngine {
        WasmtimeEngine::new(Some(dir.to_owned()))
    }
}

/// [`MEMORY_LIMIT`] and the other store limits, plus the freeze: once set, the memory may not grow past it
struct Limits {
    store: StoreLimits,
    frozen: Option<usize>,
}

impl ResourceLimiter for Limits {
    fn memory_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> Result<bool> {
        if self.frozen.is_some_and(|f| desired > f) {
            return Ok(false);
        }
        self.store.memory_growing(current, desired, maximum)
    }

    fn table_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> Result<bool> {
        self.store.table_growing(current, desired, maximum)
    }

    fn instances(&self) -> usize {
        self.store.instances()
    }

    fn tables(&self) -> usize {
        self.store.tables()
    }

    fn memories(&self) -> usize {
        self.store.memories()
    }
}

struct State {
    name: String,
    limits: Limits,
    log: LogLimiter,
    start: Instant,
}

/// one running module in its own store
pub struct Wasmtime {
    store: Store<State>,
    memory: Memory,
    f_abi: TypedFunc<(), u32>,
    f_io: TypedFunc<(), u32>,
    f_new: TypedFunc<(), ()>,
    f_layout: Option<TypedFunc<(), u32>>,
    /// `dm_process()` / `dm_control()`: no argument, the header carries the frames
    f_process: Option<TypedFunc<(), ()>>,
    f_control: Option<TypedFunc<(), ()>>,
    f_asset_buf: Option<TypedFunc<(u32, u32), u32>>,
    start: Instant,
}

/// an optional export with its signature checked (a wrong one is an error, a missing one `None`)
fn typed<P: WasmParams, R: WasmResults>(store: &mut Store<State>, f: Option<Func>, name: &str) -> Result<Option<TypedFunc<P, R>>> {
    f.map(|f| f.typed::<P, R>(&*store).with_context(|| format!("the export `{name}` has the wrong signature"))).transpose()
}

impl Backend for Wasmtime {
    type Engine = WasmtimeEngine;
    type Compiled = Module;

    fn compile(engine: &WasmtimeEngine, wasm: &[u8], hash: &str) -> Result<(Module, bool, Duration)> {
        let t = Instant::now();
        if let Some(dir) = &engine.cache {
            let file = dir.join(format!("{hash}.cwasm"));
            if file.is_file() {
                // SAFETY: the file is our own `Module::serialize` output under a SHA-256 name in the deck's cache
                // directory; wasmtime also checks its version and compile settings and refuses a mismatch
                match unsafe { Module::deserialize_file(&engine.engine, &file) } {
                    Ok(m) => return Ok((m, true, t.elapsed())),
                    Err(e) => tracing::warn!("module cache {}: {e:#} – compiling again", file.display()),
                }
            }
        }
        let module = Module::new(&engine.engine, wasm).context("compiling the module")?;
        if let Some(dir) = &engine.cache {
            let write = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(dir.join(format!("{hash}.cwasm")), module.serialize().map_err(std::io::Error::other)?));
            if let Err(e) = write {
                tracing::warn!("module cache {}: {e}", dir.display());
            }
        }
        Ok((module, false, t.elapsed()))
    }

    fn imports(m: &Module) -> Vec<(String, String)> {
        m.imports().map(|i| (i.module().to_owned(), i.name().to_owned())).collect()
    }

    fn has_export(m: &Module, name: &str) -> bool {
        m.get_export(name).is_some()
    }

    fn instantiate(module: &Module, name: &str) -> Result<Wasmtime> {
        let engine = module.engine();
        let mut linker: Linker<State> = Linker::new(engine);
        linker.func_wrap("env", "deck_log", |mut c: Caller<'_, State>, level: u32, ptr: u32, len: u32| {
            let text = c
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .and_then(|m| m.data(&c).get(ptr as usize..(ptr as usize).saturating_add(len as usize)).map(|b| String::from_utf8_lossy(b).into_owned()));
            let Some(text) = text else { return };
            let st = c.data_mut();
            let now = st.start.elapsed().as_secs_f64();
            let name = st.name.clone();
            log_line(&mut st.log, now, &name, level, &text);
        })?;
        let start = Instant::now();
        let state = State {
            name: name.to_owned(),
            limits: Limits { store: StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).instances(1).tables(10_000).memories(1).build(), frozen: None },
            log: LogLimiter::default(),
            start,
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(1000);
        let instance = linker.instantiate(&mut store, module).context("instantiating the module")?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("no memory export"))?;
        let mut get = |n: &str| instance.get_func(&mut store, n);
        let (abi, io, new, layout, process, control, asset_buf) =
            (get("dm_abi"), get("dm_io"), get("dm_new"), get("dm_layout"), get("dm_process"), get("dm_control"), get("dm_asset_buf"));
        let need = |f: Option<Func>, n: &str| f.ok_or_else(|| anyhow!("no `{n}` export"));
        Ok(Wasmtime {
            f_abi: need(abi, "dm_abi")?.typed(&store)?,
            f_io: need(io, "dm_io")?.typed(&store)?,
            f_new: need(new, "dm_new")?.typed(&store)?,
            f_layout: typed(&mut store, layout, "dm_layout")?,
            f_process: typed(&mut store, process, "dm_process")?,
            f_control: typed(&mut store, control, "dm_control")?,
            f_asset_buf: typed(&mut store, asset_buf, "dm_asset_buf")?,
            store,
            memory,
            start,
        })
    }

    fn call(&mut self, export: Export, args: &[Arg]) -> Result<u32, Fault> {
        let u = |i: usize| match args.get(i) {
            Some(Arg::U32(v)) => *v,
            _ => 0,
        };
        let missing = || Fault::Trap(format!("the module has no `{}` export", export.name()));
        let store = &mut self.store;
        let r = match export {
            Export::Abi => self.f_abi.call(store, ()),
            Export::Io => self.f_io.call(store, ()),
            Export::Layout => self.f_layout.as_ref().ok_or_else(missing)?.call(store, ()),
            Export::New => self.f_new.call(store, ()).map(|()| 0),
            Export::Process | Export::Control => {
                let f = if export == Export::Process { &self.f_process } else { &self.f_control };
                f.as_ref().ok_or_else(missing)?.call(store, ()).map(|()| 0)
            }
            Export::AssetBuf => self.f_asset_buf.as_ref().ok_or_else(missing)?.call(store, (u(0), u(1))),
        };
        r.map_err(|e| {
            if e.downcast_ref::<wasmtime::Trap>().is_some_and(|t| *t == wasmtime::Trap::Interrupt) {
                Fault::Budget
            } else {
                Fault::Trap(format!("{e:#}").lines().next().unwrap_or("trap").to_owned())
            }
        })
    }

    fn memory_size(&self) -> usize {
        self.memory.data_size(&self.store)
    }

    fn write(&mut self, off: usize, bytes: &[u8]) {
        self.memory.data_mut(&mut self.store)[off..off + bytes.len()].copy_from_slice(bytes);
    }

    fn read(&self, off: usize, out: &mut [u8]) {
        out.copy_from_slice(&self.memory.data(&self.store)[off..off + out.len()]);
    }

    fn set_budget(&mut self, ms: u64) {
        self.store.set_epoch_deadline(ms);
    }

    fn freeze_memory(&mut self) {
        let size = self.memory_size();
        self.store.data_mut().limits.frozen = Some(size);
    }

    fn clock_us(&self) -> u64 {
        self.start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
    }
}
