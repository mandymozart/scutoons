//! An instrument (#191, research/INSTRUMENTS.md §1): a patch of the scene's node graph – `[[node]]` + `[[link]]` in
//! scene.rs's forms, node ids local to the instrument – with one audio output (the track's `out`) and the control inputs
//! it shows (`expose`, the being's 3D buttons, the knobs, the mapping targets). `patch` applies a folded preset to it.
use super::preset::{Mapping, Resolved};
use super::{Rights, Text};
use crate::graph::{Graph, Kind, PortType};
use crate::scene::{split_ref, LinkDef, NodeDef, Scene};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What an instrument is for – an Ableton track's role. A scene object's sound (#210) is an instrument too.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    Drum,
    Bass,
    Lead,
    Pad,
    Voice,
    Sampler,
    /// noise, field recordings, an object's hiss (the incense)
    Texture,
    /// an effect on a send / the master
    Fx,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub id: String,
    pub kind: InstrumentKind,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub name: Text,
    #[serde(rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link", skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkDef>,
    /// the track's output: an audio output `node.port` (a bare node = its `out`)
    pub out: String,
    /// the parameters it shows; empty = every control input
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<Expose>,
    #[serde(default)]
    pub rights: Rights,
}

/// A shown parameter: a control input `node.port`, its label, and a narrower range than params.rs's (optional).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expose {
    pub path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub label: Text,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
}

/// An instrument with a preset applied: the nodes (values written as their params) and links to build, and the
/// mappings to bind. `graph()` checks it as a graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Patch {
    pub nodes: Vec<NodeDef>,
    pub links: Vec<LinkDef>,
    pub out: String,
    pub mappings: Vec<Mapping>,
}

/// Kinds a scene owns once (graph.rs's sinks + the clock): never inside an instrument – the rig / scene adds them.
pub(crate) const SCENE_ONLY: [Kind; 6] = [Kind::Screen, Kind::AudioOut, Kind::Out, Kind::Tempo, Kind::Space, Kind::Outlet];

impl Patch {
    pub fn graph(&self) -> Result<Graph> {
        let mut scene = Scene::parse("")?;
        scene.nodes = self.nodes.clone();
        scene.links = self.links.clone();
        Graph::build(&scene)
    }

    /// a node's param as written (a preset's value lands here)
    pub fn value(&self, path: &str) -> Option<f32> {
        let (node, port) = split_ref(path);
        let v = self.nodes.iter().find(|n| n.id == node)?.params.get(port)?;
        v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32))
    }
}

impl Instrument {
    /// The instrument checked on its own: a valid graph without scene-only kinds, `out` an audio output, every exposed
    /// path a control input.
    pub fn check(&self) -> Result<Graph> {
        let unpatched =
            Patch { nodes: self.nodes.clone(), links: self.links.clone(), out: self.out.clone(), mappings: vec![] };
        let graph = unpatched.graph().with_context(|| format!("instrument '{}'", self.id))?;
        if let Some(n) = graph.nodes.iter().find(|n| SCENE_ONLY.contains(&n.kind)) {
            bail!("instrument '{}': a {:?} node belongs to the scene, not to an instrument ({})", self.id, n.kind, n.id);
        }
        let (node, port) = split_ref(&self.out);
        let ok = graph.nodes.iter().find(|n| n.id == node).and_then(|n| {
            let i = n.kind.output(port)?;
            (n.kind.outputs()[i].ty == PortType::Audio).then_some(())
        });
        if ok.is_none() {
            bail!("instrument '{}': out '{}' is not an audio output", self.id, self.out);
        }
        for e in &self.expose {
            self.control_input(&e.path)?;
        }
        // an events node holds a patch (deck-events): it must build – known generators, edges, `out`, at most 4 knobs
        for n in graph.nodes.iter().filter(|n| n.kind == Kind::Events) {
            let mut names = deck_events::Names::new();
            let notes = deck_events::patch::NoteNames::new(&mut names);
            deck_events::patch::Patch::from_table(&n.params)
                .and_then(|p| p.build(notes, &mut names).map(|_| ()))
                .map_err(|e| anyhow::anyhow!("instrument '{}': events node '{}': {e}", self.id, n.id))?;
        }
        Ok(graph)
    }

    /// `node.port` → the node's kind, when `port` is one of its control inputs
    pub fn control_input(&self, path: &str) -> Result<Kind> {
        let (node, port) = split_ref(path);
        let Some(n) = self.nodes.iter().find(|n| n.id == node) else {
            bail!("instrument '{}': no node '{node}' ({path})", self.id)
        };
        let Some(kind) = Kind::parse(&n.kind) else { bail!("instrument '{}': unknown kind '{}'", self.id, n.kind) };
        match kind.input(port).map(|i| kind.inputs()[i].ty) {
            Some(PortType::Control) => Ok(kind),
            _ => bail!("instrument '{}': {path} is not a control input of {kind:?}", self.id),
        }
    }

    /// Apply a folded preset (preset.rs `Resolved`): the routing changes first (an added link replaces the input's
    /// link, `unlink` removes it), then each value is written as its node's param. A value on a linked input is an
    /// error – the link would win (feel.rs's nudges have the same rule).
    pub fn patch(&self, preset: &Resolved) -> Result<Patch> {
        if preset.instrument != self.id {
            bail!("a preset of '{}' applied to the instrument '{}'", preset.instrument, self.id);
        }
        for target in preset.unlink.iter().chain(preset.links.keys()) {
            let (node, port) = split_ref(target);
            let known = self.nodes.iter().find(|n| n.id == node).and_then(|n| Kind::parse(&n.kind)?.input(port));
            if known.is_none() {
                bail!("instrument '{}': no input {target} to re-route", self.id);
            }
        }
        let mut links: Vec<LinkDef> = self
            .links
            .iter()
            .filter(|l| !preset.unlink.contains(&canonical(&l.to)) && !preset.links.contains_key(&canonical(&l.to)))
            .cloned()
            .collect();
        links.extend(preset.links.values().cloned());
        let mut nodes = self.nodes.clone();
        for (path, v) in &preset.values {
            self.control_input(path)?;
            if !v.is_finite() {
                bail!("instrument '{}': {path} = {v} is not a number", self.id);
            }
            if links.iter().any(|l| canonical(&l.to) == canonical(path)) {
                bail!("instrument '{}': {path} is linked – a preset value needs an unlinked input", self.id);
            }
            let (node, port) = split_ref(path);
            let n = nodes.iter_mut().find(|n| n.id == node).expect("checked by control_input");
            n.params.insert(port.to_string(), toml::Value::Float(f64::from(*v)));
        }
        for m in preset.mappings.values() {
            self.control_input(&m.target)?;
        }
        let patch =
            Patch { nodes, links, out: self.out.clone(), mappings: preset.mappings.values().cloned().collect() };
        patch.graph().with_context(|| format!("instrument '{}' with its preset", self.id))?;
        Ok(patch)
    }
}

/// `node` → `node.out`, so a link's `to` compares with a path
pub(crate) fn canonical(r: &str) -> String {
    let (n, p) = split_ref(r);
    format!("{n}.{p}")
}
