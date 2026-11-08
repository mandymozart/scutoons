//! A scene for the browser deck (#191, P6 #320, #360): live-core runs deck-engine – the same engine as the native deck
//! – so `lower` only splits the scene, by the one rule every build uses (deck-core's `carried_part`) over the modules the
//! browser's engine carries (`BROWSER_MODULES`). The other nodes (a module the wasm build does not carry yet) go to the
//! page with the links that touch them. The result is checked by building it as a graph.
use crate::graph::Graph;
use crate::scene::{LinkDef, NodeDef, Scene};
use anyhow::{anyhow, Result};
use deck_core::stage::BROWSER_MODULES;
use serde::Serialize;

/// whether the browser's engine carries a module
pub fn carries(feature: &str) -> bool {
    BROWSER_MODULES.contains(&feature)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Engine {
    /// the scene live-core loads (`[[node]]` + `[[link]]`)
    pub scene: String,
    /// the nodes the engine does not carry – the page's
    pub page: Vec<NodeDef>,
    /// the links that touch a page node
    pub dropped: Vec<LinkDef>,
    /// what the browser does not play of it, in words (the page's status; '' = all of it plays)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// the status of the nodes the engine does not carry: a module (a code rig's policy) has no runtime in the browser yet
fn note(page: &[NodeDef]) -> String {
    let (modules, others): (Vec<&NodeDef>, Vec<&NodeDef>) = page.iter().partition(|n| n.kind.starts_with("module:"));
    let named = |list: &[&NodeDef]| list.iter().map(|n| format!("{} ({})", n.id, n.kind)).collect::<Vec<_>>().join(", ");
    let mut out = Vec::new();
    if !modules.is_empty() {
        out.push(format!("sound: this browser deck has no module runtime yet (#290), so {} does not play here – the tracks play without it, a native deck plays it", named(&modules)));
    }
    if !others.is_empty() {
        out.push(format!("sound: not in the browser yet – {}", named(&others)));
    }
    out.join(" · ")
}

#[derive(Serialize)]
struct Doc<'a> {
    node: &'a [NodeDef],
    link: &'a [LinkDef],
}

/// `scene` (a flattened rig, `Rig::flatten`, or a deck's scene) split for live-core
pub fn lower(scene: &Scene) -> Result<Engine> {
    let (part, page, dropped) = scene.carried_part(carries);
    let text = toml::to_string(&Doc { node: &part.nodes, link: &part.links })?;
    let parsed = Scene::parse(&text).map_err(|e| anyhow!("the lowered scene: {e}"))?;
    Graph::build(&parsed).map_err(|e| anyhow!("the lowered scene doesn't build: {e}"))?;
    Ok(Engine { scene: text, note: note(&page), page, dropped })
}
