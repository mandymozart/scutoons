//! module-bench-web (#306): the suite of module-bench in a browser / Node worker. The host's logic (deck-module-host:
//! descriptor, layout, io staging, strict / show, restart) runs here in Rust; the platform's own `WebAssembly` runs the
//! modules behind the `bench.*` imports of www/bench.ts – compile, instantiate, call an export, copy bytes in and out of
//! a module's memory, set / read the gas global, the clock. That JS holds no logic: the shape of the `web` backend #290
//! would ship. With the `wasmi` feature the same wasm also carries wasmi (`web-wasmi`): the interpreter inside the worker.
//!
//! Exports (raw, no wasm-bindgen): `bench_alloc(len) -> ptr`, `bench_set_module(kind, ptr, len)` (kind: 0 fm16, 1 walk,
//! 2 analog, 3 stress), `bench_run(runtime_ptr, runtime_len, cfg_ptr, cfg_len) -> len` (the RuntimeResult as JSON),
//! `bench_table(ptr, len) -> len` (a Results list as the Markdown table and gates), `bench_out() -> ptr` (the last answer).
use anyhow::{anyhow, Result};
use deck_module_host::{Arg, Backend, Export, Fault};
use module_bench::{report, run_suite, Config, Modules, Results, Runtime};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
mod js {
    #[link(wasm_import_module = "bench")]
    extern "C" {
        pub fn compile(ptr: *const u8, len: u32) -> i32;
        pub fn module_info(id: i32, out: *mut u8, cap: u32) -> u32;
        pub fn instantiate(id: i32, gas: u32) -> i32;
        pub fn drop_instance(inst: i32);
        /// ≥ 0 the export's result · -1 a trap (`last_error`) · -2 out of gas
        pub fn call(inst: i32, export: u32, a0: u32, a1: u32) -> f64;
        pub fn mem_size(inst: i32) -> f64;
        pub fn mem_write(inst: i32, off: u32, src: *const u8, len: u32);
        pub fn mem_read(inst: i32, off: u32, dst: *mut u8, len: u32);
        pub fn set_gas(inst: i32, gas: f64);
        pub fn gas_left(inst: i32) -> f64;
        pub fn last_error(out: *mut u8, cap: u32) -> u32;
        /// the watchdog's start: right before the runaway's call
        pub fn mark();
    }
}

/// natively the imports do not exist: the crate only checks there (`cargo check --workspace`)
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::missing_safety_doc)]
mod js {
    pub unsafe fn compile(_: *const u8, _: u32) -> i32 {
        -1
    }
    pub unsafe fn module_info(_: i32, _: *mut u8, _: u32) -> u32 {
        0
    }
    pub unsafe fn instantiate(_: i32, _: u32) -> i32 {
        -1
    }
    pub unsafe fn drop_instance(_: i32) {}
    pub unsafe fn call(_: i32, _: u32, _: u32, _: u32) -> f64 {
        -1.0
    }
    pub unsafe fn mem_size(_: i32) -> f64 {
        0.0
    }
    pub unsafe fn mem_write(_: i32, _: u32, _: *const u8, _: u32) {}
    pub unsafe fn mem_read(_: i32, _: u32, _: *mut u8, _: u32) {}
    pub unsafe fn set_gas(_: i32, _: f64) {}
    pub unsafe fn gas_left(_: i32) -> f64 {
        0.0
    }
    pub unsafe fn last_error(_: *mut u8, _: u32) -> u32 {
        0
    }
    pub unsafe fn mark() {}
}

fn last_error() -> String {
    let mut buf = vec![0u8; 512];
    // SAFETY: the import writes at most `cap` bytes into our buffer
    let n = unsafe { js::last_error(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    String::from_utf8_lossy(&buf[..n.min(buf.len())]).into_owned()
}

/// gas before the calibration: effectively none
const UNCALIBRATED: f64 = 1e15;

pub struct WebEngine {
    gas: bool,
    per_ms: Arc<AtomicU64>,
}

pub struct WebCompiled {
    id: i32,
    imports: Vec<(String, String)>,
    exports: Vec<String>,
    gas: bool,
    per_ms: Arc<AtomicU64>,
}

/// one module instance on the platform's WebAssembly
pub struct Web {
    inst: i32,
    gas: bool,
    per_ms: Arc<AtomicU64>,
    budget: f64,
    last: Option<u64>,
    /// the browser cannot refuse a module's `memory.grow`: the host sees it after the call and faults the instance
    frozen: Option<usize>,
}

impl Drop for Web {
    fn drop(&mut self) {
        // SAFETY: a plain import
        unsafe { js::drop_instance(self.inst) }
    }
}

impl Runtime for Web {
    fn units(&self) -> Option<u64> {
        self.last
    }

    fn calibrate(engine: &WebEngine, units_per_ms: f64) {
        engine
            .per_ms
            .store(units_per_ms.to_bits(), Ordering::Relaxed);
    }
}

impl Backend for Web {
    type Engine = WebEngine;
    type Compiled = WebCompiled;

    fn compile(
        engine: &WebEngine,
        wasm: &[u8],
        _hash: &str,
    ) -> Result<(WebCompiled, bool, Duration)> {
        let t = module_bench::now_us();
        // SAFETY: JS copies `len` bytes from our memory
        let id = unsafe { js::compile(wasm.as_ptr(), wasm.len() as u32) };
        let us = module_bench::now_us() - t;
        if id < 0 {
            return Err(anyhow!("compiling the module: {}", last_error()));
        }
        let mut buf = vec![0u8; 4096];
        // SAFETY: JS writes at most `cap` bytes
        let n = unsafe { js::module_info(id, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        let info: serde_json::Value = serde_json::from_slice(&buf[..n.min(buf.len())])?;
        let strs = |v: &serde_json::Value| {
            v.as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let imports = info["imports"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(strs)
                    .map(|p| {
                        (
                            p.first().cloned().unwrap_or_default(),
                            p.get(1).cloned().unwrap_or_default(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let exports = strs(&info["exports"]);
        Ok((
            WebCompiled {
                id,
                imports,
                exports,
                gas: engine.gas,
                per_ms: engine.per_ms.clone(),
            },
            false,
            Duration::from_secs_f64(us / 1e6),
        ))
    }

    fn imports(c: &WebCompiled) -> Vec<(String, String)> {
        c.imports.clone()
    }

    fn has_export(c: &WebCompiled, name: &str) -> bool {
        c.exports.iter().any(|e| e == name)
    }

    fn instantiate(c: &WebCompiled, _name: &str) -> Result<Web> {
        // SAFETY: a plain import
        let inst = unsafe { js::instantiate(c.id, u32::from(c.gas)) };
        if inst < 0 {
            return Err(anyhow!("instantiating the module: {}", last_error()));
        }
        Ok(Web {
            inst,
            gas: c.gas,
            per_ms: c.per_ms.clone(),
            budget: UNCALIBRATED,
            last: None,
            frozen: None,
        })
    }

    fn call(&mut self, export: Export, args: &[Arg]) -> Result<u32, Fault> {
        let u = |i: usize| match args.get(i) {
            Some(Arg::U32(v)) => *v,
            _ => 0,
        };
        let index = Export::ALL.iter().position(|e| *e == export).unwrap_or(0) as u32;
        if self.gas {
            // SAFETY: plain imports
            unsafe { js::set_gas(self.inst, self.budget) };
        }
        // SAFETY: a plain import
        let r = unsafe { js::call(self.inst, index, u(0), u(1)) };
        if self.gas {
            // SAFETY: a plain import
            let left = unsafe { js::gas_left(self.inst) };
            self.last = Some(if r == -2.0 {
                self.budget as u64
            } else {
                (self.budget - left).max(0.0) as u64
            });
        }
        if r == -2.0 {
            return Err(Fault::Budget);
        }
        if r < 0.0 {
            return Err(Fault::Trap(last_error()));
        }
        if let Some(f) = self.frozen {
            if self.memory_size() > f {
                return Err(Fault::Trap(format!("the memory grew past its freeze ({} → {} bytes; the browser cannot refuse memory.grow)", f, self.memory_size())));
            }
        }
        Ok(r as u32)
    }

    fn memory_size(&self) -> usize {
        // SAFETY: a plain import
        unsafe { js::mem_size(self.inst) as usize }
    }

    fn write(&mut self, off: usize, bytes: &[u8]) {
        // SAFETY: JS copies `len` bytes from our memory into the module's (the host checked the range)
        unsafe { js::mem_write(self.inst, off as u32, bytes.as_ptr(), bytes.len() as u32) }
    }

    fn read(&self, off: usize, out: &mut [u8]) {
        // SAFETY: JS copies `len` bytes from the module's memory into ours
        unsafe { js::mem_read(self.inst, off as u32, out.as_mut_ptr(), out.len() as u32) }
    }

    fn set_budget(&mut self, ms: u64) {
        let per_ms = f64::from_bits(self.per_ms.load(Ordering::Relaxed));
        self.budget = if per_ms > 0.0 {
            (ms as f64 * per_ms).min(UNCALIBRATED)
        } else {
            UNCALIBRATED
        };
    }

    fn freeze_memory(&mut self) {
        self.frozen = Some(self.memory_size());
    }

    fn clock_us(&self) -> u64 {
        module_bench::now_us() as u64
    }
}

thread_local! {
    static MODULES: RefCell<Modules> = RefCell::new(Modules::default());
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn answer(s: String) -> u32 {
    let n = s.len() as u32;
    OUT.with(|o| *o.borrow_mut() = s.into_bytes());
    n
}

/// # Safety
/// `ptr`/`len` name bytes JS wrote into a buffer from [`bench_alloc`].
unsafe fn text(ptr: *const u8, len: u32) -> String {
    String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len as usize)).into_owned()
}

/// a buffer of `len` bytes for JS to fill (taken back by `bench_set_module`, or leaked: a bench)
#[no_mangle]
pub extern "C" fn bench_alloc(len: u32) -> *mut u8 {
    let mut v = vec![0u8; len as usize];
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr`/`len` come from [`bench_alloc`]`(len)`, filled by JS.
#[no_mangle]
pub unsafe extern "C" fn bench_set_module(kind: u32, ptr: *mut u8, len: u32) {
    let bytes = Vec::from_raw_parts(ptr, len as usize, len as usize);
    let name = Modules::NAMES
        .get(kind as usize)
        .copied()
        .unwrap_or("stress");
    MODULES.with(|m| m.borrow_mut().set(name, bytes));
}

fn run(runtime: &str, cfg: &str) -> Result<String> {
    let cfg: Config = serde_json::from_str(cfg)?;
    let m = MODULES.with(|m| m.borrow().clone());
    let r = match runtime {
        #[cfg(feature = "wasmi")]
        "web-wasmi" => run_suite::<module_bench::wasmi_backend::Wasmi>(
            &module_bench::wasmi_backend::WasmiEngine::default(),
            runtime,
            "worker",
            &m,
            &cfg,
            Some("fuel"),
        ),
        "web-gas" => run_suite::<Web>(
            &WebEngine {
                gas: true,
                per_ms: Arc::default(),
            },
            runtime,
            "worker",
            &m,
            &cfg,
            Some("gas"),
        ),
        "web-loops" => run_suite::<Web>(
            &WebEngine {
                gas: true,
                per_ms: Arc::default(),
            },
            runtime,
            "worker",
            &m,
            &cfg,
            Some("loop counter"),
        ),
        // web-nan / web-simd: the NaN pass's and the simd128 build's modules (#396), run like `web`
        "web" | "web-nan" | "web-simd" => run_suite::<Web>(
            &WebEngine {
                gas: false,
                per_ms: Arc::default(),
            },
            runtime,
            "worker",
            &m,
            &cfg,
            None,
        ),
        // never returns: the watchdog's case
        "web-runaway" => {
            return module_bench::hang::<Web>(
                &WebEngine {
                    gas: false,
                    per_ms: Arc::default(),
                },
                &m.stress,
                || unsafe { js::mark() },
            )
            .map(|_| String::new())
        }
        x => return Err(anyhow!("unknown runtime {x}")),
    };
    Ok(serde_json::to_string(&r)?)
}

/// # Safety
/// both strings from [`bench_alloc`] buffers JS filled.
#[no_mangle]
pub unsafe extern "C" fn bench_run(
    rt: *const u8,
    rt_len: u32,
    cfg: *const u8,
    cfg_len: u32,
) -> u32 {
    let (rt, cfg) = (text(rt, rt_len), text(cfg, cfg_len));
    answer(run(&rt, &cfg).unwrap_or_else(|e| {
        serde_json::json!({ "runtime": rt, "error": format!("{e:#}") }).to_string()
    }))
}

/// # Safety
/// the JSON (a list of results) from a [`bench_alloc`] buffer JS filled.
#[no_mangle]
pub unsafe extern "C" fn bench_table(ptr: *const u8, len: u32) -> u32 {
    let all: Vec<Results> = serde_json::from_str(&text(ptr, len)).unwrap_or_default();
    answer(report::markdown(&all))
}

#[no_mangle]
pub extern "C" fn bench_out() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}
