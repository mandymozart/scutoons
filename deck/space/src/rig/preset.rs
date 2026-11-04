//! Presets and their lineage (#191, research/INSTRUMENTS.md §2): a preset holds only what it changes against its parent
//! (or, without one, against the instrument as patched) – values, routing changes, mappings. A derivative is a child
//! preset; `PresetBook::resolve` folds the chain root → leaf into one `Resolved` the instrument applies.
use super::instrument::canonical;
use super::{Hold, Right, Rights, Text};
use crate::scene::LinkDef;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    pub instrument: String,
    /// the preset this one derives from (same instrument); none = a root
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub name: Text,
    /// free tags: a character's trait, a mood, a persona, a genre …
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// control input `node.port` → its value
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f32>,
    /// inputs whose link is removed (`node.port`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unlink: Vec<String>,
    /// links added; one into an input that already has a link replaces it
    #[serde(default, rename = "link", skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkDef>,
    /// control sources bound to parameters (#209, #212), saved with the preset
    #[serde(default, rename = "mapping", skip_serializing_if = "Vec::is_empty")]
    pub mappings: Vec<Mapping>,
    #[serde(default)]
    pub rights: Rights,
}

/// A control source bound to a parameter. `source`: `midi:<channel>:cc:<n>` · `face:<blendshape>` (`face:jawOpen`) ·
/// `motion:<axis>` (`motion:tilt.x`) · `pad:<control>` (`pad:left.x`) · `stick:<left|right>.<x|y>`; its 0..1 (or −1..1)
/// lands on `target` across `range` (default: the parameter's range, params.rs).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub source: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
}

/// A preset chain folded into what the instrument applies (instrument.rs `patch`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolved {
    pub instrument: String,
    pub values: BTreeMap<String, f32>,
    pub unlink: BTreeSet<String>,
    /// added links by their input (`node.port`)
    pub links: BTreeMap<String, LinkDef>,
    /// by (source, target): a child's mapping with the same pair replaces its parent's
    pub mappings: BTreeMap<(String, String), Mapping>,
}

impl Resolved {
    /// no preset: the instrument as it is
    pub fn bare(instrument: &str) -> Resolved {
        Resolved { instrument: instrument.to_string(), ..Default::default() }
    }

    fn fold(&mut self, p: &Preset) {
        for (k, v) in &p.values {
            self.values.insert(canonical(k), *v);
        }
        for u in &p.unlink {
            let u = canonical(u);
            self.links.remove(&u);
            self.unlink.insert(u);
        }
        for l in &p.links {
            let to = canonical(&l.to);
            self.unlink.remove(&to);
            self.links.insert(to, l.clone());
        }
        for m in &p.mappings {
            self.mappings.insert((m.source.clone(), canonical(&m.target)), m.clone());
        }
    }
}

/// The presets a deck knows (core + owned + lent – how it holds them is the carrier bag's, #192), by id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetBook {
    pub presets: BTreeMap<String, Preset>,
}

impl PresetBook {
    pub fn new(presets: impl IntoIterator<Item = Preset>) -> PresetBook {
        PresetBook { presets: presets.into_iter().map(|p| (p.id.clone(), p)).collect() }
    }

    pub fn get(&self, id: &str) -> Result<&Preset> {
        match self.presets.get(id) {
            Some(p) => Ok(p),
            None => bail!("no preset '{id}'"),
        }
    }

    /// `id` and its ancestors, root first. Refused: an unknown parent, a cycle, a parent of another instrument.
    pub fn lineage(&self, id: &str) -> Result<Vec<&Preset>> {
        let mut chain = vec![self.get(id)?];
        while let Some(parent) = &chain.last().expect("never empty").parent {
            let child = chain.last().expect("never empty");
            if chain.iter().any(|p| &p.id == parent) {
                bail!("preset {id}: its lineage loops at \"{parent}\"");
            }
            let Some(p) = self.presets.get(parent) else { bail!("preset {}: unknown preset \"{parent}\" (its parent)", child.id) };
            if p.instrument != child.instrument {
                bail!("preset {} ({}) derives from \"{}\" of another instrument ({})", child.id, child.instrument, p.id, p.instrument);
            }
            chain.push(p);
        }
        chain.reverse();
        Ok(chain)
    }

    /// the chain folded root → leaf
    pub fn resolve(&self, id: &str) -> Result<Resolved> {
        let chain = self.lineage(id)?;
        let mut r = Resolved::bare(&chain[0].instrument);
        for p in chain {
            r.fold(p);
        }
        Ok(r)
    }

    /// A derivative of `parent` holding only the values that differ from the parent's folded ones – "what changed".
    /// Needs the parent's `derive` right for how it is held.
    pub fn derive(&self, parent: &str, id: &str, held: Hold, values: &BTreeMap<String, f32>) -> Result<Preset> {
        let p = self.get(parent)?;
        if !p.rights.allows(held, Right::Derive) {
            bail!("preset '{parent}' held {held:?} may not be derived from");
        }
        if self.presets.contains_key(id) {
            bail!("a preset '{id}' exists already");
        }
        let base = self.resolve(parent)?;
        let changed = values
            .iter()
            .map(|(k, v)| (canonical(k), *v))
            .filter(|(k, v)| base.values.get(k) != Some(v))
            .collect();
        Ok(Preset {
            id: id.to_string(),
            instrument: p.instrument.clone(),
            parent: Some(parent.to_string()),
            name: Text::new(),
            tags: p.tags.clone(),
            values: changed,
            unlink: vec![],
            links: vec![],
            mappings: vec![],
            rights: p.rights.clone(),
        })
    }
}
