//! A flattened rig for the browser deck (#191, P6 #320): live-core runs deck-engine – the same engine as the native deck
//! – so `lower` only splits the scene. The kinds the browser's engine carries (`ENGINE_KINDS`) stay as they are: mixers,
//! cv links, waveforms, the walk and events hosts, the fft and the being's `space` node pass through unchanged. Every
//! other node (a kind the wasm build does not carry yet) goes to the page with the links that touch it. The result is
//! checked by building it as a graph.
use crate::graph::Graph;
use crate::scene::{split_ref, LinkDef, NodeDef, Scene};
use anyhow::{anyhow, Result};
use serde::Serialize;

/// The kinds live-core's engine plays (the allowlist of plan P6): every audio and control kind deck-engine builds
/// without a platform service, plus `space` (the being: its inputs are control values the page reads back). Not yet:
/// `chapter_music` (needs decoded audio – deck-media, plan 2.9), `module:*` (the browser's module backend, P9b), and the
/// picture kinds (the page draws its own picture).
pub const ENGINE_KINDS: &[&str] = &[
    "osc", "noise", "fm", "lfo", "smooth", "mul", "add", "tempo", "click", "seq", "walk", "events", "mixer", "fft", "audio_out", "bus", "out", "pan",
    "downmix", "feedback", "space",
];

/// whether the browser's engine plays `kind`
pub fn carried(kind: &str) -> bool {
    ENGINE_KINDS.contains(&kind)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Engine {
    /// the scene live-core loads (`[[node]]` + `[[link]]`)
    pub scene: String,
    /// the nodes the engine does not carry – the page's
    pub page: Vec<NodeDef>,
    /// the links that touch a page node
    pub dropped: Vec<LinkDef>,
}

#[derive(Serialize)]
struct Doc<'a> {
    node: &'a [NodeDef],
    link: &'a [LinkDef],
}

/// `scene` (a flattened rig, `Rig::flatten`) split for live-core
pub fn lower(scene: &Scene) -> Result<Engine> {
    let (nodes, page): (Vec<NodeDef>, Vec<NodeDef>) = scene.nodes.iter().cloned().partition(|n| carried(&n.kind));
    let kept = |r: &str| nodes.iter().any(|n| n.id == split_ref(r).0);
    let (links, dropped): (Vec<LinkDef>, Vec<LinkDef>) = scene.links.iter().cloned().partition(|l| kept(&l.from) && kept(&l.to));
    let text = toml::to_string(&Doc { node: &nodes, link: &links })?;
    let parsed = Scene::parse(&text).map_err(|e| anyhow!("the lowered scene: {e}"))?;
    Graph::build(&parsed).map_err(|e| anyhow!("the lowered scene doesn't build: {e}"))?;
    Ok(Engine { scene: text, page, dropped })
}
