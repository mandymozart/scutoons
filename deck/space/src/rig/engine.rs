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
    Ok(Engine { scene: text, page, dropped })
}
