//! A flattened rig for the browser's sound engine (#191, research/INSTRUMENTS.md §7): live-core runs deck-core's batch
//! evaluator, which knows only its batch kinds (`deck_core::batch::kind_ports`: lfo, smooth, mul, add, osc, fm) and
//! sums its audio nodes itself. `lower` keeps what it can play, folds the mixers' strip gains into a `mul` before each
//! audio node's gain (the knob's path then turns that mul – `aliases`), drops the scene's plumbing (mixer, audio_out,
//! fft – live-core analyses its own output –, space) and hands the page the rest: the nodes it plays itself (noise …)
//! with their strip gain, and the links the engine can't carry (audio links like seq → fm.cv, links to the page's
//! nodes or the being). The result is checked by building it as a batch graph.
use crate::scene::{split_ref, LinkDef, NodeDef, Scene};
use anyhow::{anyhow, Result};
use deck_core::batch::{kind_ports, BatchGraph};
use serde::Serialize;
use std::collections::BTreeMap;

/// kinds the batch engine replaces by itself (the sum, the analyser) or the page draws (the being)
const PLUMBING: [&str; 4] = ["mixer", "audio_out", "fft", "space"];

/// A node the engine can't play: the page plays it (Web Audio) at `gain` × its own gain.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageNode {
    #[serde(flatten)]
    pub node: NodeDef,
    /// its mixer strip's gain (the chain's product)
    pub gain: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Engine {
    /// the batch scene live-core loads (`[[node]]` + `[[link]]`)
    pub scene: String,
    /// knob path → the engine path to set instead (a strip-gained node's `gain` → its strip's `a`)
    pub aliases: BTreeMap<String, String>,
    /// the nodes the page plays
    pub page: Vec<PageNode>,
    /// the links the engine doesn't carry
    pub dropped: Vec<LinkDef>,
}

#[derive(Serialize)]
struct Doc<'a> {
    node: &'a [NodeDef],
    link: &'a [LinkDef],
}

/// the gain from `id`'s output to the end of the mixer chain (1 = not mixed, or unity)
fn strip(scene: &Scene, id: &str) -> f32 {
    let mut gain = 1.0;
    let mut at = id.to_string();
    // a chain of mixers ends at audio_out; bounded by the node count (graph.rs has refused cycles already)
    for _ in 0..scene.nodes.len() {
        let next = scene.links.iter().find_map(|l| {
            let (fnode, _) = split_ref(&l.from);
            let (tnode, tport) = split_ref(&l.to);
            let mixer = scene.nodes.iter().find(|n| n.id == tnode && n.kind == "mixer")?;
            let k = tport.strip_prefix("in")?;
            (fnode == at).then(|| {
                let g = mixer.params.get(&format!("gain{k}")).and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
                (mixer.id.clone(), g)
            })
        });
        match next {
            Some((m, g)) => {
                gain *= g;
                at = m;
            }
            None => break,
        }
    }
    gain
}

/// `scene` (a flattened rig, `Rig::flatten`) lowered for live-core
pub fn lower(scene: &Scene) -> Result<Engine> {
    let mut nodes = Vec::new();
    let mut page = Vec::new();
    let mut aliases = BTreeMap::new();
    let mut links = Vec::new();
    let mut dropped = Vec::new();
    let batch = |id: &str| scene.nodes.iter().find(|n| n.id == id).and_then(|n| kind_ports(&n.kind));
    for n in &scene.nodes {
        if PLUMBING.contains(&n.kind.as_str()) {
            continue;
        }
        let Some((ports, audio)) = kind_ports(&n.kind) else {
            page.push(PageNode { node: n.clone(), gain: strip(scene, &n.id) });
            continue;
        };
        // only the kind's ports: other params (an LFO's shape) mean nothing to the batch engine
        let params = n.params.iter().filter(|(k, _)| ports.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone()));
        let mut node = NodeDef { id: n.id.clone(), kind: n.kind.clone(), params: params.collect() };
        let g = strip(scene, &n.id);
        if audio && g != 1.0 && ports.contains(&"gain") {
            let id = format!("{}_strip", n.id);
            let mut t = toml::Table::new();
            // the knob's value as written (no f32 round trip), else the engine's default gain
            t.insert("a".into(), n.params.get("gain").cloned().unwrap_or(toml::Value::Float(1.0)));
            t.insert("b".into(), toml::Value::Float(f64::from(g)));
            node.params.remove("gain");
            nodes.push(NodeDef { id: id.clone(), kind: "mul".into(), params: t });
            links.push(LinkDef { from: id.clone(), to: format!("{}.gain", n.id), scale: 1.0, offset: 0.0 });
            aliases.insert(format!("{}.gain", n.id), format!("{id}.a"));
        }
        nodes.push(node);
    }
    for l in &scene.links {
        let (fnode, _) = split_ref(&l.from);
        let (tnode, tport) = split_ref(&l.to);
        let kind_of = |id: &str| scene.nodes.iter().find(|n| n.id == id).map(|n| n.kind.as_str());
        if kind_of(fnode).is_some_and(|k| k == "mixer") || kind_of(tnode).is_some_and(|k| k == "mixer" || k == "audio_out") {
            continue;
        }
        match (batch(fnode), batch(tnode)) {
            (Some((_, false)), Some((ports, _))) if ports.contains(&tport) => {
                // into a strip-gained gain: into its strip's `a`
                let to = aliases.get(&format!("{tnode}.{tport}")).cloned().unwrap_or_else(|| l.to.clone());
                links.push(LinkDef { to, ..l.clone() });
            }
            _ => dropped.push(l.clone()),
        }
    }
    let text = toml::to_string(&Doc { node: &nodes, link: &links })?;
    let def = deck_core::SceneDef::parse(&text).map_err(|e| anyhow!("the lowered scene: {e}"))?;
    BatchGraph::build(&def, 48_000.0).map_err(|e| anyhow!("the lowered scene doesn't build: {e}"))?;
    Ok(Engine { scene: text, aliases, page, dropped })
}
