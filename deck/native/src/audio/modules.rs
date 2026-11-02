//! The native deck's module library as deck-engine's `ModuleSource` (#280, feature `modules`): `module:<name>` nodes
//! open an `Instance` of the process's `Library` (module_host.rs: wasmtime behind deck-module-host's `Backend`). The
//! engine drives the instance; the library, the compile cache and the files stay here.
use crate::module_host::{note_silenced, Instance, Library};
use anyhow::{anyhow, Result};
use deck_engine::{ModuleSource, Opened};
use deck_space::modreg::{ModuleKind, PREFIX};

/// the process's library (`Library::global`, loaded from `DECK_MODULES`) as the engine's module source
pub struct Global;

impl ModuleSource for Global {
    fn open(&self, kind: ModuleKind, rate: f64, seed: u64) -> Result<Opened> {
        let loaded = Library::global()
            .of_kind(kind)
            .ok_or_else(|| anyhow!("the module '{}' is not loaded (DECK_MODULES)", kind.spec().kind.trim_start_matches(PREFIX)))?;
        let instance = Instance::new(&loaded, rate, seed)?.into_host();
        Ok(Opened { instance: Box::new(instance), inputs: loaded.layout.inputs.clone(), outputs: loaded.layout.outputs.clone(), params: loaded.desc.params.len() })
    }

    fn silenced(&self) {
        note_silenced();
    }
}
