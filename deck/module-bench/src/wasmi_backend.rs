//! wasmi 2 behind deck-module-host's `Backend` (#306): the interpreter the issue weighs against the JITs – one VM,
//! the same on a deck and in a browser worker (this file compiles into the web crate's wasm too), fuel as the budget
//! and as the deterministic cost unit. A bench backend: wasmi is not a dependency of any production crate.
//! The budget is a time like wasmtime's epoch: `set_budget(ms)` gives ms × the fuel a millisecond (calibrated from the
//! fm16 run, [`crate::Runtime::calibrate`]); out of fuel = `Fault::Budget`. Eager compilation (the compile time is all
//! of it, nothing deferred to the first call), validation on.
use crate::{now_us, Runtime};
use anyhow::{anyhow, Context, Result};
use deck_module_host::{log_line, Arg, Backend, Export, Fault, LogLimiter, MEMORY_LIMIT};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasmi::{
    Caller, CompilationMode, Config, Engine, Linker, Memory, Module, ResourceLimiter, Store,
    StoreLimits, StoreLimitsBuilder, TrapCode, TypedFunc,
};
use wasmi_core::LimiterError;

/// fuel before the calibration (the first fm16 run): effectively none
const UNCALIBRATED: u64 = u64::MAX / 4;

pub struct WasmiEngine {
    engine: Engine,
    /// fuel a millisecond (f64 bits); 0 = not calibrated
    per_ms: Arc<AtomicU64>,
}

impl Default for WasmiEngine {
    fn default() -> Self {
        let mut config = Config::default();
        config.consume_fuel(true);
        config.compilation_mode(CompilationMode::Eager);
        WasmiEngine {
            engine: Engine::new(&config),
            per_ms: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub struct Compiled {
    module: Module,
    per_ms: Arc<AtomicU64>,
}

struct Limits {
    store: StoreLimits,
    frozen: Option<usize>,
}

impl ResourceLimiter for Limits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, LimiterError> {
        if self.frozen.is_some_and(|f| desired > f) {
            return Ok(false);
        }
        self.store.memory_growing(current, desired, maximum)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, LimiterError> {
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
}

pub struct Wasmi {
    store: Store<State>,
    memory: Memory,
    funcs: [Option<TypedFunc<(), u32>>; 3],
    calls: [Option<TypedFunc<(), ()>>; 3],
    asset_buf: Option<TypedFunc<(u32, u32), u32>>,
    per_ms: Arc<AtomicU64>,
    budget: u64,
    last: Option<u64>,
}

impl Runtime for Wasmi {
    fn units(&self) -> Option<u64> {
        self.last
    }

    fn calibrate(engine: &WasmiEngine, units_per_ms: f64) {
        engine
            .per_ms
            .store(units_per_ms.to_bits(), Ordering::Relaxed);
    }
}

impl Backend for Wasmi {
    type Engine = WasmiEngine;
    type Compiled = Compiled;

    fn compile(
        engine: &WasmiEngine,
        wasm: &[u8],
        _hash: &str,
    ) -> Result<(Compiled, bool, Duration)> {
        let t = now_us();
        let module =
            Module::new(&engine.engine, wasm).map_err(|e| anyhow!("compiling the module: {e}"))?;
        Ok((
            Compiled {
                module,
                per_ms: engine.per_ms.clone(),
            },
            false,
            Duration::from_secs_f64((now_us() - t) / 1e6),
        ))
    }

    fn imports(c: &Compiled) -> Vec<(String, String)> {
        c.module
            .imports()
            .map(|i| (i.module().to_owned(), i.name().to_owned()))
            .collect()
    }

    fn has_export(c: &Compiled, name: &str) -> bool {
        c.module.get_export(name).is_some()
    }

    fn instantiate(c: &Compiled, name: &str) -> Result<Wasmi> {
        let engine = c.module.engine();
        let mut linker: Linker<State> = Linker::new(engine);
        linker
            .func_wrap(
                "env",
                "deck_log",
                |c: Caller<'_, State>, level: u32, ptr: u32, len: u32| {
                    let text = c
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .and_then(|m| {
                            m.data(&c)
                                .get(ptr as usize..(ptr as usize).saturating_add(len as usize))
                                .map(|b| String::from_utf8_lossy(b).into_owned())
                        });
                    let Some(text) = text else { return };
                    let mut c = c;
                    let st = c.data_mut();
                    let name = st.name.clone();
                    log_line(&mut st.log, now_us() / 1e6, &name, level, &text);
                },
            )
            .map_err(|e| anyhow!("{e}"))?;
        let state = State {
            name: name.to_owned(),
            limits: Limits {
                store: StoreLimitsBuilder::new()
                    .memory_size(MEMORY_LIMIT)
                    .instances(1)
                    .tables(10_000)
                    .memories(1)
                    .build(),
                frozen: None,
            },
            log: LogLimiter::default(),
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(UNCALIBRATED).map_err(|e| anyhow!("{e}"))?;
        let instance = linker
            .instantiate_and_start(&mut store, &c.module)
            .map_err(|e| anyhow!("instantiating the module: {e}"))?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| anyhow!("no memory export"))?;
        let get_u32 = |store: &Store<State>, n: &str| {
            instance
                .get_func(store, n)
                .map(|f| f.typed::<(), u32>(store))
                .transpose()
                .with_context(|| format!("`{n}` has the wrong signature"))
        };
        let get_unit = |store: &Store<State>, n: &str| {
            instance
                .get_func(store, n)
                .map(|f| f.typed::<(), ()>(store))
                .transpose()
                .with_context(|| format!("`{n}` has the wrong signature"))
        };
        let funcs = [
            get_u32(&store, "dm_abi")?,
            get_u32(&store, "dm_io")?,
            get_u32(&store, "dm_layout")?,
        ];
        let calls = [
            get_unit(&store, "dm_new")?,
            get_unit(&store, "dm_process")?,
            get_unit(&store, "dm_control")?,
        ];
        let asset_buf = instance
            .get_func(&store, "dm_asset_buf")
            .map(|f| f.typed::<(u32, u32), u32>(&store))
            .transpose()
            .context("`dm_asset_buf` has the wrong signature")?;
        Ok(Wasmi {
            store,
            memory,
            funcs,
            calls,
            asset_buf,
            per_ms: c.per_ms.clone(),
            budget: UNCALIBRATED,
            last: None,
        })
    }

    fn call(&mut self, export: Export, args: &[Arg]) -> Result<u32, Fault> {
        let u = |i: usize| match args.get(i) {
            Some(Arg::U32(v)) => *v,
            _ => 0,
        };
        let missing = || Fault::Trap(format!("the module has no `{}` export", export.name()));
        let _ = self.store.set_fuel(self.budget);
        let store = &mut self.store;
        let r = match export {
            Export::Abi => self.funcs[0].as_ref().ok_or_else(missing)?.call(store, ()),
            Export::Io => self.funcs[1].as_ref().ok_or_else(missing)?.call(store, ()),
            Export::Layout => self.funcs[2].as_ref().ok_or_else(missing)?.call(store, ()),
            Export::New => self.calls[0]
                .as_ref()
                .ok_or_else(missing)?
                .call(store, ())
                .map(|()| 0),
            Export::Process => self.calls[1]
                .as_ref()
                .ok_or_else(missing)?
                .call(store, ())
                .map(|()| 0),
            Export::Control => self.calls[2]
                .as_ref()
                .ok_or_else(missing)?
                .call(store, ())
                .map(|()| 0),
            Export::AssetBuf => self
                .asset_buf
                .as_ref()
                .ok_or_else(missing)?
                .call(store, (u(0), u(1))),
        };
        self.last = self
            .store
            .get_fuel()
            .ok()
            .map(|left| self.budget.saturating_sub(left));
        r.map_err(|e| {
            if e.as_trap_code() == Some(TrapCode::OutOfFuel) {
                Fault::Budget
            } else {
                Fault::Trap(e.to_string().lines().next().unwrap_or("trap").to_owned())
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
        let per_ms = f64::from_bits(self.per_ms.load(Ordering::Relaxed));
        self.budget = if per_ms > 0.0 {
            (ms as f64 * per_ms).min(UNCALIBRATED as f64) as u64
        } else {
            UNCALIBRATED
        };
    }

    fn freeze_memory(&mut self) {
        let size = self.memory_size();
        self.store.data_mut().limits.frozen = Some(size);
    }

    fn clock_us(&self) -> u64 {
        now_us() as u64
    }
}
