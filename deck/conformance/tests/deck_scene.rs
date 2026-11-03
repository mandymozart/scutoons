//! (#360) The browser deck plays the X1's scene: the cathedral file the native deck boots with, through the core – the
//! same graph (hash) as the native deck builds of it for the browser's modules, the same samples, the seq's arp. No copy
//! of the scene in TypeScript: the page gets it from deck-space (`Played::deck_scene`, space-web's `deck_scene`).
mod common;
use common::*;
use deck_core::stage::BROWSER_MODULES;
use deck_native::audio::{Build, Engine};
use deck_native::graph::Graph;
use deck_native::scene::Scene;
use deck_space::rig::Played;

/// the graph's shape: its nodes (id, kind, params) and links (ends, scale, offset) – FNV-1a over their text
fn graph_hash(g: &Graph) -> u64 {
    let mut text = String::new();
    for n in &g.nodes {
        text.push_str(&format!("{}:{}:{:?};", n.id, n.kind.name(), n.params));
    }
    for l in &g.links {
        text.push_str(&format!("{:?}>{:?}*{}+{};", l.from, l.to, l.scale.to_bits(), l.offset.to_bits()));
    }
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// the native deck's engine over `graph`, `frames` frames in quanta; the left channel
fn native_render(graph: &Graph, frames: usize) -> Vec<f32> {
    let (mut engine, _handles) = Engine::build_with(graph, RATE, 1.0, Build::detect()).expect("engine builds");
    let mut out = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; 256];
    while out.len() < frames {
        engine.render(&mut buf, 2, |x| x);
        out.extend(buf.chunks_exact(2).map(|f| f[0]));
    }
    out.truncate(frames);
    out
}

#[test]
fn the_browser_plays_the_x1s_cathedral() {
    // the native deck: the file it boots with (`--scene cathedral`), its part for the browser's modules
    let file = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../native/scenes/cathedral.toml")).expect("the X1's scene");
    let native = Scene::parse(&file).unwrap().carried_part(|f| BROWSER_MODULES.contains(&f)).0;
    let native_graph = Graph::build(&native).unwrap();
    // the browser deck: what the page gets (deck-space, embedded in the wasm) and hands to live-core
    let played = Played::deck_scene().unwrap();
    assert_eq!(played.scene, file, "the page's scene is the X1's file, byte for byte");
    let browser_graph = Graph::build(&Scene::parse(&played.engine.scene).unwrap()).unwrap();
    assert_eq!(graph_hash(&browser_graph), graph_hash(&native_graph), "native and browser load the same scene to the same graph");
    // live-core loads it (lc_load's host) and its table is that graph's nodes
    let mut host = live_core::Host::new(RATE, 2);
    let text = played.engine.scene.as_bytes();
    host.inbox(text.len())[..text.len()].copy_from_slice(text);
    let n = host.load(text.len());
    let answer = String::from_utf8_lossy(&host.outbox()[..n.unsigned_abs() as usize]).into_owned();
    assert!(n > 0, "live-core loads the deck's scene: {answer}");
    let table: serde_json::Value = serde_json::from_str(&answer).unwrap();
    let ids: Vec<&str> = table["nodes"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids, native_graph.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>());
    // the arp: the cathedral's seq with its steps, no page melody (the old `walk`)
    let seq = native_graph.nodes.iter().find(|n| n.id == "seq").expect("the seq");
    assert_eq!(seq.f32s("steps"), [50.0, 0.0, 57.0, 55.0, 53.0, 0.0, 52.0, 50.0]);
    assert!(!ids.contains(&"melody") && !browser_graph.nodes.iter().any(|n| n.kind.name() == "walk"));
    // and the same samples: the native engine and live-core, 4 s
    let frames = 4 * 48_000;
    let stage = native_render(&native_graph, frames);
    let core = core_render(&played.engine.scene, 128, frames);
    assert!(peak(&stage) > 1e-3, "the cathedral sounds");
    let d = diff(&stage, &core);
    assert!(d.exact, "the browser deck plays the native deck's samples: {d:?}");
}
