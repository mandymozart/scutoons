//! A flattened rig for the browser's sound engine (#191, research/INSTRUMENTS.md §7): live-core runs deck-core's batch
//! evaluator, which knows only its batch kinds (`deck_core::batch::kind_ports`: lfo, smooth, mul, add, osc, fm) and
//! sums its audio nodes itself. `lower` keeps what it can play, folds the mixers' strip gains into a `mul` before each
//! audio node's gain (the knob's path then turns that mul – `aliases`), drops the scene's plumbing (mixer, audio_out,
//! fft – live-core analyses its own output –, space) and hands the page the rest: the nodes it plays itself (noise …)
//! with their strip gain, and the links the engine can't carry (audio links like seq → fm.cv, links to the page's
//! nodes or the being). A `walk` (#236) linked into a voice's cv becomes a `[[walk]]` that plays the voice's freq
//! (live-core's walks.rs), an `events` node (a patch) an `[[events]]`. The result is checked by building it as a batch
//! graph.
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
    /// the batch scene live-core loads (`[[node]]` + `[[link]]` + `[[walk]]`)
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
    #[serde(skip_serializing_if = "<[WalkDoc]>::is_empty")]
    walk: &'a [WalkDoc],
    #[serde(skip_serializing_if = "<[toml::Table]>::is_empty")]
    events: &'a [toml::Table],
}

/// An `events` node (#236) for live-core (its walks.rs `EventsDef`): its params – the patch (`generator`, `edge`,
/// `out`, `knobs`), the knobs' values a–d, `base`, `bpm` – as written, plus its id and the voice input it plays.
fn events_doc(n: &NodeDef, target: String) -> toml::Table {
    let mut t = n.params.clone();
    t.insert("id".into(), toml::Value::String(n.id.clone()));
    t.insert("target".into(), toml::Value::String(target));
    t
}

/// A `walk` (#236) for live-core (its walks.rs `WalkDef`): the batch graph has no cv inputs, so a walk linked into a
/// voice's `cv` plays that voice's `freq`. Its clock is its own `bpm` (the browser has no tempo clock).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WalkDoc {
    pub id: String,
    pub target: String,
    pub seed: u64,
    pub base: f32,
    pub octaves: f32,
    pub length: f32,
    pub spread: f32,
    pub rate: f32,
    pub glide: f32,
    pub bpm: f64,
}

/// the walk `n` playing `target`, its params as written or the kind's defaults
fn walk_doc(n: &NodeDef, target: String) -> WalkDoc {
    let num = |k: &str| n.params.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)));
    let knob = |k: &str| num(k).map_or_else(|| crate::params::default_for(crate::graph::Kind::Walk, k), |v| v as f32);
    WalkDoc {
        id: n.id.clone(),
        target,
        seed: num("seed").map_or(1, |v| v as u64),
        base: num("base").map_or(147.0, |v| v as f32),
        octaves: num("octaves").map_or(2.0, |v| v as f32),
        length: num("length").map_or(1.0, |v| v as f32),
        spread: knob("spread"),
        rate: knob("rate"),
        glide: knob("glide"),
        bpm: num("bpm").unwrap_or(120.0),
    }
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
    // walks linked into a batch voice's cv play its freq (live-core's walks); the link is theirs
    let mut walks = Vec::new();
    let mut walked = Vec::new();
    let mut events = Vec::new();
    let mut hosted = Vec::new();
    for n in scene.nodes.iter().filter(|n| n.kind == "walk" || n.kind == "events") {
        let into = scene.links.iter().find(|l| split_ref(&l.from).0 == n.id && split_ref(&l.to).1 == "cv");
        let voice = into.map(|l| split_ref(&l.to).0).filter(|v| batch(v).is_some_and(|(ports, audio)| audio && ports.contains(&"freq")));
        if let (Some(l), Some(v)) = (into, voice) {
            if n.kind == "walk" {
                walks.push(walk_doc(n, format!("{v}.freq")));
            } else {
                events.push(events_doc(n, format!("{v}.freq")));
            }
            walked.push(l.clone());
            hosted.push(n.id.clone());
        }
    }
    for n in &scene.nodes {
        if PLUMBING.contains(&n.kind.as_str()) || hosted.contains(&n.id) {
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
        if walked.contains(l) {
            continue;
        }
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
    let text = toml::to_string(&Doc { node: &nodes, link: &links, walk: &walks, events: &events })?;
    let def = deck_core::SceneDef::parse(&text).map_err(|e| anyhow!("the lowered scene: {e}"))?;
    BatchGraph::build(&def, 48_000.0).map_err(|e| anyhow!("the lowered scene doesn't build: {e}"))?;
    Ok(Engine { scene: text, aliases, page, dropped })
}
