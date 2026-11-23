//! `scene:cloud-light` in the toon `sandbox` (#805): the document server/tests/fixtures/scenes/cloud-light.json – the
//! cloud-storm rig placed (`rig:item.cloud`, the catalog's rows: server/tests/fixtures/rigs/cloud-storm.json), held and
//! tinted storm-blue by its holder's values, the wash and the strobe in its `outputs` – resolved by the core as a deck
//! resolves it: the sandbox deck/native/scenes/sandbox/cloud-light.toml's graph and light. The migration
//! 2026-11-24-cloud-light-sandbox publishes this document as it stands.
use deck_space::scene::Scene;
use deck_space::scene_document::SceneDocument;
use deck_space::scene_resolve::{core_rigs, no_sets, resolve};
use std::collections::{BTreeMap, BTreeSet};

const DOCUMENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/cloud-light.json");
const RIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/cloud-storm.json");
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../native/scenes/sandbox/cloud-light.toml");
/// the holder of the placed rig: its nodes are the sandbox's under `cloud_`
const HOLDER: &str = "cloud";

/// the document, its text the canonical one (`UPDATE_FIXTURES=1` writes it)
fn document() -> SceneDocument {
    let stored = std::fs::read_to_string(DOCUMENT).unwrap().replace('\r', "");
    let doc = SceneDocument::parse_json(&stored).unwrap();
    let canonical = doc.to_json().unwrap() + "\n";
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(DOCUMENT, &canonical).unwrap();
    } else {
        assert_eq!(stored, canonical, "cloud-light.json is not canonical: UPDATE_FIXTURES=1 cargo test -p deck-space --test cloud_light");
    }
    assert_eq!(doc.check(), Vec::<String>::new());
    doc
}

/// the rig's module kinds, as a deck that loaded them knows them
fn register_modules() {
    use deck_module::Describe;
    for json in [cloud_module::Cloud::DESCRIPTOR, sampler_module::Sampler::DESCRIPTOR, mixer_module::Mixer::DESCRIPTOR] {
        let d: deck_catalog::Descriptor = serde_json::from_str(json).unwrap();
        deck_space::modreg::register(&d.name, &deck_space::modreg::ModuleDef::of(&d));
    }
}

fn resolved() -> Scene {
    register_modules();
    let rig: deck_space::rig::Bundle = serde_json::from_str(&std::fs::read_to_string(RIG).unwrap()).unwrap();
    let doc = document();
    assert_eq!((doc.id.as_str(), doc.toons.as_slice()), ("cloud-light", ["sandbox".to_string()].as_slice()));
    let rigs = |s: &str, v: Option<&str>| if s == "item.cloud" { Some(rig.clone()) } else { core_rigs(s, v) };
    resolve(&doc, &BTreeMap::new(), false, &rigs, &no_sets).unwrap().scene
}

/// the node ids every `from`, `gain.from` and `trigger` of the outputs names (`<node>.<port>`)
fn named(outputs: &toml::Table) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    fn walk(v: &toml::Value, out: &mut BTreeSet<String>) {
        match v {
            toml::Value::Table(t) => {
                for (k, x) in t {
                    match (k.as_str(), x.as_str()) {
                        ("from" | "trigger", Some(path)) => {
                            out.insert(path.to_string());
                        }
                        _ => walk(x, out),
                    }
                }
            }
            toml::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&toml::Value::Table(outputs.clone()), &mut out);
    out
}

/// The placed rig is the sandbox's flattened rig: its cloud held and tinted, its puffs the library's set of models, the samplers'
/// sounds; the outputs drive the wash and the strobe from the cloud's ports, which the resolved graph has.
#[test]
fn the_scene_places_the_cloud_storm_with_its_light() {
    let s = resolved();
    let graph = deck_space::graph::Graph::build(&s).unwrap_or_else(|e| panic!("{e:#}"));
    let cloud = s.nodes.iter().find(|n| n.kind == "module:cloud").expect("the cloud");
    for (k, v) in [("hold", 1.0), ("hue", 0.62), ("tint", 60.0)] {
        assert_eq!(cloud.params[k].as_float().map(|x| x as f32), Some(v), "{k}");
    }
    let set: Vec<&str> = cloud.params["assets"]["puff"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(set, ["scutoons/3d/cloud-three-blobs.glb", "scutoons/3d/cloud-four-blobs.glb"], "the owner's two blob clouds");
    let outputs = s.outputs.as_ref().expect("its light");
    let paths = named(outputs);
    assert_eq!(paths.len(), 5, "red, green, blue, pulse, gate: {paths:?}");
    for p in &paths {
        let (node, port) = p.split_once('.').unwrap();
        let n = graph.nodes.iter().find(|n| n.id == node).unwrap_or_else(|| panic!("{p}: no node {node} in {:?}", graph.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()));
        assert_eq!(n.id, cloud.id, "{p}: the cloud's");
        assert!(["red", "green", "blue", "pulse", "gate"].contains(&port), "{p}");
    }
    // the sandbox's light, in the placed rig's paths (under the holder's prefix)
    let sandbox = Scene::parse(&std::fs::read_to_string(SANDBOX).unwrap()).unwrap();
    let held: BTreeSet<String> = named(sandbox.outputs.as_ref().unwrap()).iter().map(|p| format!("{HOLDER}_{p}")).collect();
    assert_eq!(held, paths, "the sandbox's light");
    let fixtures = |t: &toml::Table| t["fixture"].as_array().unwrap().iter().map(|f| f["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(fixtures(outputs), fixtures(sandbox.outputs.as_ref().unwrap()), "the wash and the strobe");
}

/// a deck plays the same storm from the API scene as from the sandbox: the same module nodes (under the holder's prefix)
/// with the same values (a holder's value is an f32) and cables
#[test]
fn the_rig_s_nodes_are_the_sandbox_s() {
    let s = resolved();
    let sandbox = Scene::parse(&std::fs::read_to_string(SANDBOX).unwrap()).unwrap();
    let f32s = |t: &toml::Table| t.iter().map(|(k, v)| (k.clone(), v.as_float().map_or_else(|| v.to_string(), |x| (x as f32).to_string()))).collect::<BTreeMap<_, _>>();
    let modules = |s: &Scene, prefix: &str| {
        s.nodes.iter().filter(|n| n.kind.starts_with("module:")).map(|n| (format!("{prefix}{}", n.id), n.kind.clone(), f32s(&n.params))).collect::<Vec<_>>()
    };
    let held = format!("{HOLDER}_");
    assert_eq!(modules(&s, ""), modules(&sandbox, &held));
    let cables = |s: &Scene, prefix: &str, own: &str| {
        s.links.iter().filter(|l| l.from.starts_with(own) && l.to.starts_with(own)).map(|l| (format!("{prefix}{}", l.from), format!("{prefix}{}", l.to))).collect::<BTreeSet<_>>()
    };
    let rig = cables(&s, "", &format!("{held}storm_"));
    assert_eq!(rig.len(), 4, "the cloud's two notes, the samplers into the mixer: {rig:?}");
    assert_eq!(rig, cables(&sandbox, &held, "storm_"));
}
