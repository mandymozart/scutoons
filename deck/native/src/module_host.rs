//! The module host on the native deck (#280, feature `modules`, research/RIG-EDITING.md §4): a thin wrapper around
//! `deck-module-host` (#305) with its wasmtime backend. The host logic – descriptor and layout, the `Io` staging, the log
//! limiter, the time budget, silencing a trapped instance – lives in that crate; here are what needs the stage:
//! - `Library` loads a `.wasm` (compile cached in memory and, with `DECK_MODULE_CACHE`, on disk) and registers its
//!   ports as the node kind `module:<name>` (deck-space's modreg.rs). `autoload` loads every `*.wasm` of `DECK_MODULES`
//!   before the stage builds a scene.
//! - `Instance` is one running module (the crate's, bound to the process's library).
//!
//! Member code is never run on the server – this runs on a deck, in the browser's engine, or in a test.
use anyhow::Result;
use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
use deck_module_host::Loaded as HostLoaded;
use deck_space::modreg::{self, ModuleKind};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub use deck_module_host::{InBind, Layout, LogLimiter, OutBind, MEMORY_LIMIT};

/// A loaded module: checked, compiled, registered as a node kind. Cheap to share.
pub struct Loaded {
    host: Arc<HostLoaded<Wasmtime>>,
    pub kind: ModuleKind,
}

impl Deref for Loaded {
    type Target = HostLoaded<Wasmtime>;
    fn deref(&self) -> &Self::Target {
        &self.host
    }
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.host.fmt(f)
    }
}

pub struct Library {
    engine: WasmtimeEngine,
    by_hash: Mutex<HashMap<String, Arc<Loaded>>>,
    by_name: Mutex<HashMap<String, Arc<Loaded>>>,
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
        Library { engine: WasmtimeEngine::new(cache), by_hash: Mutex::new(HashMap::new()), by_name: Mutex::new(HashMap::new()) }
    }

    /// Loads a module from its wasm bytes: descriptor first (nothing runs), then the compile (cached), then the kind.
    pub fn load(&self, wasm: &[u8]) -> Result<Arc<Loaded>> {
        let host = HostLoaded::<Wasmtime>::load(&self.engine, wasm)?;
        if let Some(l) = self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).get(&host.hash) {
            return Ok(l.clone());
        }
        // the node kind: deck-engine's one mapping from the host's layout to the graph (#342)
        let kind = deck_engine::register_module(&host.desc, &host.layout);
        let hash = host.hash.clone();
        let loaded = Arc::new(Loaded { host: Arc::new(host), kind });
        self.by_hash.lock().unwrap_or_else(|e| e.into_inner()).insert(hash, loaded.clone());
        self.by_name.lock().unwrap_or_else(|e| e.into_inner()).insert(loaded.desc.name.to_string(), loaded.clone());
        Ok(loaded)
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

/// One running module (the host crate's instance on wasmtime).
pub struct Instance(deck_module_host::Instance<Wasmtime>);

impl Instance {
    /// Instantiates `loaded` for `rate` Hz with `seed` (the same seed plays the same on every deck), strict (tests).
    pub fn new(loaded: &Arc<Loaded>, rate: f64, seed: u64) -> Result<Instance> {
        Instance::with_mode(loaded, rate, seed, deck_module_host::Mode::Strict)
    }

    /// … in `mode` (a deck plays a show: a trap restarts the instance, #342)
    pub fn with_mode(loaded: &Arc<Loaded>, rate: f64, seed: u64, mode: deck_module_host::Mode) -> Result<Instance> {
        deck_module_host::Instance::new(&loaded.host, deck_module_host::Setup::new(rate, seed).mode(mode)).map(Instance)
    }
}

impl Instance {
    /// the host crate's instance itself (what deck-engine drives through `ModuleInstance`)
    pub fn into_host(self) -> deck_module_host::Instance<Wasmtime> {
        self.0
    }
}

impl Deref for Instance {
    type Target = deck_module_host::Instance<Wasmtime>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Instance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// set once any instance has been silenced (the HUD and the tests can look)
pub static ANY_SILENCED: AtomicBool = AtomicBool::new(false);

pub fn note_silenced() {
    ANY_SILENCED.store(true, Ordering::Relaxed);
}
