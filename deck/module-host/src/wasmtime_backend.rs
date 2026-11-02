//! The wasmtime backend (what the native deck runs): the engine with its 1 ms epoch ticker – the time budget is
//! wasmtime's epoch interruption –, the compile cache (in memory by the caller, on disk here, keyed by the wasm's SHA-256)
//! and one store per instance with the memory capped at [`MEMORY_LIMIT`].
use crate::{log_line, Arg, Backend, Export, Fault, LogLimiter, MEMORY_LIMIT};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use wasmtime::{Caller, Config, Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

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

struct State {
    name: String,
    limits: StoreLimits,
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
    f_set_param: TypedFunc<(u32, f32), ()>,
    f_process: TypedFunc<u32, ()>,
    f_control: TypedFunc<u32, ()>,
    start: Instant,
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
            limits: StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).instances(1).tables(10_000).memories(1).build(),
            log: LogLimiter::default(),
            start,
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(1000);
        let instance = linker.instantiate(&mut store, module).context("instantiating the module")?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("no memory export"))?;
        Ok(Wasmtime {
            f_abi: instance.get_typed_func(&mut store, "dm_abi")?,
            f_io: instance.get_typed_func(&mut store, "dm_io")?,
            f_new: instance.get_typed_func(&mut store, "dm_new")?,
            f_set_param: instance.get_typed_func(&mut store, "dm_set_param")?,
            f_process: instance.get_typed_func(&mut store, "dm_process")?,
            f_control: instance.get_typed_func(&mut store, "dm_control")?,
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
        let r = match export {
            Export::Abi => self.f_abi.call(&mut self.store, ()),
            Export::Io => self.f_io.call(&mut self.store, ()),
            Export::New => self.f_new.call(&mut self.store, ()).map(|()| 0),
            Export::SetParam => {
                let v = match args.get(1) {
                    Some(Arg::F32(v)) => *v,
                    _ => 0.0,
                };
                self.f_set_param.call(&mut self.store, (u(0), v)).map(|()| 0)
            }
            Export::Process => self.f_process.call(&mut self.store, u(0)).map(|()| 0),
            Export::Control => self.f_control.call(&mut self.store, u(0)).map(|()| 0),
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

    fn clock_us(&self) -> u64 {
        self.start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
    }
}
