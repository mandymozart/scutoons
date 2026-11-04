//! The native deck's module library as deck-engine's `ModuleSource` (#280, feature `modules`): `module:<name>` nodes
//! open an `Instance` of the process's `Library` (module_host.rs: wasmtime behind deck-module-host's `Backend`). The
//! engine drives the instance; the library, the compile cache and the files stay here. A deck plays a show: an ABI v2
//! instance opens in the show mode (a trap restarts it, #342) and `spawn_loader` runs the restarts' rebuilds on a thread.
use crate::module_host::{note_silenced, Instance, Library};
use anyhow::{anyhow, Result};
use deck_engine::{ModuleSource, Opened, Restarts};
use deck_module_host::Mode;
use deck_space::modreg::{ModuleKind, PREFIX};
use std::time::Duration;

/// the process's library (`Library::global`, loaded from `DECK_MODULES`) as the engine's module source
pub struct Global;

impl ModuleSource for Global {
    fn open(&self, kind: ModuleKind, rate: f64, seed: u64) -> Result<Opened> {
        let loaded = Library::global()
            .of_kind(kind)
            .ok_or_else(|| anyhow!("the module '{}' is not loaded (DECK_MODULES)", kind.spec().kind.trim_start_matches(PREFIX)))?;
        let instance = Instance::with_mode(&loaded, rate, seed, Mode::Show)?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }

    fn silenced(&self) {
        note_silenced();
    }
}

/// how often the loader looks for a restart to rebuild
const LOADER_TICK: Duration = Duration::from_millis(5);

/// The loader thread of a show's module restarts (#342, ABI-V2 §6): it serves the engine's restart lanes (the rebuild
/// instantiates – never on the audio thread) and ends when the engine that owns them is gone. None without modules.
pub fn spawn_loader(mut lanes: Vec<Restarts>) -> std::io::Result<Option<std::thread::JoinHandle<()>>> {
    if lanes.is_empty() {
        return Ok(None);
    }
    std::thread::Builder::new()
        .name("deck-module-loader".into())
        .spawn(move || loop {
            for l in &mut lanes {
                let ran = l.serve();
                if ran > 0 {
                    tracing::info!("module node '{}': restarted ({ran} rebuild)", l.node);
                }
            }
            if lanes.iter().all(Restarts::abandoned) {
                return;
            }
            std::thread::sleep(LOADER_TICK);
        })
        .map(Some)
}
