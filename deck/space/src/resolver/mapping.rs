//! A deck's control mappings in three layers (research/INPUT.md §6b, owner 2026-11-10): precoded (what a device type
//! does out of the box, ships with the build: [`Strength::Shipped`]) < flashed (what a show, its rigs and presets carry)
//! < local (changed on this deck, each change undoable by itself). The order is the resolver's.
//!
//! The key is the control (`midi:1:cc:74`, `pad:left.x` – preset.rs `Mapping::source`): a layer's opinion on a control
//! is everything it drives, so a learned knob replaces what the flash bound to it, and a control switched off is an
//! empty list. Resetting it lets the layer below show again.
//!
//! No caller yet (#474 slice 2): the deck keeps no mapping layers today; the input concept (#468) will hold one per deck.
use super::{Resolved, Resolver, Source, Strength};
use crate::rig::preset::Mapping;
use std::collections::BTreeMap;

/// the source id of what the build ships
pub const PRECODED: &str = "precoded";
/// the source id of this deck's own changes
pub const LOCAL: &str = "local";

/// One deck's mappings.
#[derive(Debug, Clone, Default)]
pub struct MappingLayers {
    layers: Resolver<Vec<Mapping>>,
}

fn by_control(mappings: &[Mapping]) -> BTreeMap<&str, Vec<Mapping>> {
    let mut out: BTreeMap<&str, Vec<Mapping>> = BTreeMap::new();
    for m in mappings {
        out.entry(m.source.as_str()).or_default().push(m.clone());
    }
    out
}

impl MappingLayers {
    /// the floor: what the device types do out of the box
    pub fn new(precoded: &[Mapping]) -> MappingLayers {
        let mut layers = Resolver::new();
        for (control, ms) in by_control(precoded) {
            layers.set(control, Source::new(Strength::Shipped, PRECODED), ms);
        }
        MappingLayers { layers }
    }

    /// A flash (`from`: the show or version it came from) replaces the flashed layer. Local changes stay, and are
    /// returned (the flash report lists them per deck); `force_defaults` clears them too and returns what it cleared.
    pub fn flash(&mut self, from: &str, mappings: &[Mapping], force_defaults: bool) -> Vec<String> {
        let local: Vec<String> = self.layers.keys_of(Strength::Local).map(String::from).collect();
        self.layers.clear_strength(Strength::Flashed);
        if force_defaults {
            self.layers.clear_strength(Strength::Local);
        }
        for (control, ms) in by_control(mappings) {
            self.layers.set(control, Source::new(Strength::Flashed, from), ms);
        }
        local
    }

    /// What `control` drives on this deck from now on (a learned knob, another range); every mapping's source is `control`.
    pub fn set_local(&mut self, control: &str, mappings: Vec<Mapping>) {
        debug_assert!(mappings.iter().all(|m| m.source == control), "a local change of {control} maps {control} only");
        self.layers.set(control, Source::new(Strength::Local, LOCAL), mappings);
    }

    /// `control` drives nothing on this deck
    pub fn switch_off(&mut self, control: &str) {
        self.set_local(control, Vec::new());
    }

    /// "Reset this mapping": the local change of `control` goes, the layer below shows again. False: it had none.
    pub fn reset(&mut self, control: &str) -> bool {
        self.layers.clear(control, &Source::new(Strength::Local, LOCAL)).is_some()
    }

    /// the controls changed on this deck
    pub fn local(&self) -> Vec<&str> {
        self.layers.keys_of(Strength::Local).collect()
    }

    /// What `control` drives, the layer it comes from, and what it would fall back to.
    pub fn of(&self, control: &str) -> Option<Resolved<'_, Vec<Mapping>>> {
        // the layers are standing opinions: the time does not matter
        self.layers.resolve(control, 0.0)
    }

    /// Every mapping that plays on this deck now, control by control.
    pub fn playing(&self) -> Vec<&Mapping> {
        self.layers.keys().filter_map(|c| self.of(c)).flat_map(|r| r.value.iter()).collect()
    }
}
