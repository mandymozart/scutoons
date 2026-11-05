//! #408: a code rig – catpope's mood switching as the policy module of research/RIG-MODULE-GROUPS.md §3, played against
//! the declarative catpope. The policy does not run here (a module runs in an engine; the live mood reaching it is #409):
//! its outputs per mood are the note's table, the same one the module computes.
use super::*;
use crate::graph::Graph;
use crate::rig::face::Resolved as To;
use crate::rig::{policy, Bundle, Played, PresetChoice, Route, Stream, Track, Variant};
use crate::scene::Scene;
use deck_catalog::Descriptor;
use std::collections::BTreeMap;

/// the policy's descriptor as its module document stores it (the §3 example's derive writes the same)
pub(crate) const CATPOPE_RIG: &str = r#"{
  "name": "catpope_rig", "domain": "control", "abi": 2,
  "doc": "catpope: the voice's preset and FM index, the drone's level, by mood",
  "ports": [
    { "name": "voice_preset", "dir": "out", "type": "control", "path": "voice:@preset",
      "choices": ["cathedral_voice.base", "cathedral_voice.low", "cathedral_voice.bright"] },
    { "name": "voice_index", "dir": "out", "type": "control", "path": "voice:voice.index" },
    { "name": "drone_gain", "dir": "out", "type": "control", "path": "drone:drone.gain" }
  ],
  "params": [{ "name": "heat", "min": 0.0, "max": 1.0, "default": 0.0 }]
}"#;

/// mood → (voice preset choice, FM index, drone gain): the §3 module's table (row 0 = neutral)
const MOODS: [(Option<&str>, usize, f32, f32); 6] = [
    (None, 0, 2.0, 0.1),
    (Some("thinking"), 1, 1.0, 0.1),
    (Some("sad"), 1, 1.0, 0.14),
    (Some("happy"), 2, 2.0, 0.1),
    (Some("laughing"), 2, 4.0, 0.1),
    (Some("surprised"), 2, 6.0, 0.05),
];

fn descriptor(json: &str) -> Descriptor {
    serde_json::from_str(json).unwrap()
}

fn declarative() -> Bundle {
    Bundle::core("character.catpope").unwrap().unwrap()
}

/// catpope as a code rig: the core rig's tracks, instruments and presets, its mood tables replaced by the policy
pub(crate) fn code_catpope() -> Bundle {
    with_policy(CATPOPE_RIG)
}

fn with_policy(json: &str) -> Bundle {
    let mut b = declarative();
    b.rig.moods.clear();
    let d = descriptor(json);
    b.rig.policy = Some(Policy { module: d.name.to_string(), seed: None, values: BTreeMap::new() });
    b.modules = vec![d];
    b
}

/// what the bound inputs read: every numeric param of the scene's nodes (the policy's aside), then each input a
/// policy output is bound to set to `outputs[out] · scale + offset`
fn effective(p: &Played, outputs: &BTreeMap<&str, f32>) -> BTreeMap<String, f32> {
    let mut v: BTreeMap<String, f32> = BTreeMap::new();
    for n in p.nodes.iter().filter(|n| n.id != policy::NODE) {
        for (k, x) in &n.params {
            if let Some(f) = x.as_float() {
                v.insert(format!("{}.{k}", n.id), f as f32);
            }
        }
    }
    let w = p.wiring().unwrap();
    for l in w.links.iter().filter(|l| w.nodes[l.from].id == policy::NODE) {
        assert!(l.bound, "a policy target is bound, not a scene link: {l:?}");
        v.insert(format!("{}.{}", w.nodes[l.to].id, l.input), outputs[l.out.as_str()] * l.scale + l.offset);
    }
    v
}

#[test]
fn the_code_rig_flattens_with_its_policy_node_and_target_links() {
    let b = code_catpope();
    let p = b.play(None, None).unwrap();
    let node = p.nodes.iter().find(|n| n.id == "policy").expect("the policy node");
    assert_eq!(node.kind, "module:catpope_rig");
    assert_eq!(node.params.get("character").and_then(|v| v.as_str()), Some("catpope"));
    let g = Graph::build(&Scene::parse(&p.scene).unwrap()).unwrap();
    let pi = g.nodes.iter().position(|n| n.id == "policy").unwrap();
    assert_eq!(g.nodes[pi].kind.domain(), crate::graph::Domain::Control);
    let bound: Vec<String> = g.links.iter().filter(|l| l.from.0 == pi).map(|l| format!("{}.{}", g.nodes[l.to.0].id, g.nodes[l.to.0].kind.inputs()[l.to.1].name)).collect();
    assert_eq!(bound, ["voice_voice.index", "drone_drone.gain"], "@preset is no link (the deck applies it, #410)");
    // a set target holds its input's own value until the policy writes: voice.index's own 2 (low's 1 is dropped)
    let holds: Vec<f32> = g.holds.iter().filter(|h| h.0 .0 == pi).map(|h| h.1).collect();
    assert_eq!(holds, [2.0, 0.1]);
    assert_eq!(p.presets, [PresetChoice { track: "voice".into(), path: "policy.voice_preset".into(), choices: vec!["cathedral_voice.base".into(), "cathedral_voice.low".into(), "cathedral_voice.bright".into()] }]);
    assert!(p.streamed.iter().any(|(s, k)| s == "policy.voice_preset" && *k == Stream::Control));
    let heat = p.knobs.iter().find(|k| k.path == "policy.heat").expect("the policy's param is a rig knob");
    assert_eq!((heat.min, heat.max, heat.value), (0.0, 1.0, 0.0));
    // the browser's engine does not carry modules yet: the policy is the page's there
    assert!(p.engine.page.iter().any(|n| n.id == "policy"));
    // a declarative rig flattens as before: no policy, no preset choices
    let d = declarative().play(None, None).unwrap();
    assert!(d.nodes.iter().all(|n| n.id != "policy") && d.presets.is_empty());
}

/// The acceptance of T1 without the live mood: for every mood, the code rig played with the policy's choice for the
/// voice's preset and its outputs on the bound inputs is the declarative catpope's scene – the same nodes, the same
/// links, the same value on every input.
#[test]
fn the_code_rig_plays_what_the_tables_play_for_every_mood() {
    let (code, tables) = (code_catpope(), declarative());
    let choices = &code.modules[0].ports[0].choices;
    for (mood, preset, index, drone) in MOODS {
        let want = tables.play(None, mood).unwrap();
        let chosen = BTreeMap::from([("voice".to_string(), choices[preset].to_string())]);
        let got = code.play_with(None, None, &chosen).unwrap();
        let ids = |p: &Played| p.nodes.iter().filter(|n| n.id != "policy").map(|n| (n.id.clone(), n.kind.clone())).collect::<Vec<_>>();
        assert_eq!(ids(&got), ids(&want), "{mood:?}");
        assert_eq!(got.links, want.links, "{mood:?}: the policy adds no scene link");
        let outputs = BTreeMap::from([("voice_index", index), ("drone_gain", drone)]);
        let none = BTreeMap::new();
        assert_eq!(effective(&got, &outputs), effective(&want, &none), "{mood:?}");
        assert_eq!(got.tracks.iter().map(|t| t.preset.clone()).collect::<Vec<_>>(), want.tracks.iter().map(|t| t.preset.clone()).collect::<Vec<_>>(), "{mood:?}");
    }
}

#[test]
fn a_set_target_owns_its_input_and_a_preset_value_there_is_dropped_with_a_warning() {
    let b = code_catpope();
    let lib = b.library();
    assert_eq!(b.rig.owned(&lib).unwrap().into_iter().collect::<Vec<_>>(), [("drone".to_string(), "drone.gain".to_string()), ("voice".to_string(), "voice.index".to_string())]);
    assert_eq!(b.rig.warnings(&lib).unwrap(), ["cathedral_voice.low sets voice.index, which the policy drives (voice:voice.index, set): dropped"]);
    let low = b.play_with(None, None, &BTreeMap::from([("voice".to_string(), "cathedral_voice.low".to_string())])).unwrap();
    let fm = low.nodes.iter().find(|n| n.id == "voice_voice").unwrap();
    assert_eq!(fm.params.get("index").and_then(|v| v.as_float()), Some(2.0), "low's index = 1 is dropped: the instrument's own");
    assert_eq!(fm.params.get("ratio").and_then(|v| v.as_float()), Some(0.5), "low's other values play");
    // the tables may not write an owned input: a mood value, a route
    let mut moody = code_catpope();
    moody.rig.moods.insert("laughing".into(), Variant { presets: BTreeMap::new(), values: BTreeMap::from([("voice:voice.index".into(), 4.0)]) });
    let e = moody.play(None, None).unwrap_err().to_string();
    assert!(e.contains("'voice:voice.index' is the policy's (voice_index, set): mood laughing sets it too"), "{e}");
    let mut routed = code_catpope();
    routed.rig.routes.push(Route { from: "drone:drone.out".into(), to: "voice:voice.index".into(), scale: 1.0, offset: 0.0 });
    assert!(routed.play(None, None).unwrap_err().to_string().contains("the route from drone:drone.out drives it too"));
    // the policy chooses the voice's preset: no table names one
    let mut chosen = code_catpope();
    chosen.rig.personas.insert("pope".into(), Variant { presets: BTreeMap::from([("voice".into(), "cathedral_voice.low".into())]), values: BTreeMap::new() });
    assert!(chosen.play(None, None).unwrap_err().to_string().contains("the policy chooses voice's preset (voice_preset): persona pope names one too"));
}

#[test]
fn an_add_target_layers_over_the_tables() {
    // its own name: the registry's newest kind of a name wins, and the tests run side by side
    let json = CATPOPE_RIG.replace(r#""path": "voice:voice.index" }"#, r#""path": "voice:voice.index", "mode": "add" }"#).replace("\"catpope_rig\"", "\"regtest_add_policy\"");
    let b = with_policy(&json);
    let lib = b.library();
    assert!(b.rig.warnings(&lib).unwrap().is_empty(), "nothing is dropped under an add");
    let mut moody = b.clone();
    moody.rig.moods.insert("laughing".into(), Variant { presets: BTreeMap::new(), values: BTreeMap::from([("voice:voice.index".into(), 4.0)]) });
    let p = moody.play(None, Some("laughing")).unwrap();
    let g = Graph::build(&Scene::parse(&p.scene).unwrap()).unwrap();
    let pi = g.nodes.iter().position(|n| n.id == "policy").unwrap();
    let add = g.links.iter().find(|l| l.from == (pi, 1)).unwrap();
    assert_eq!((add.scale, add.offset), (1.0, 4.0), "the table's value is the base, the policy adds to it");
    let low = b.play_with(None, None, &BTreeMap::from([("voice".to_string(), "cathedral_voice.low".to_string())])).unwrap();
    assert_eq!(low.nodes.iter().find(|n| n.id == "voice_voice").unwrap().params.get("index").and_then(|v| v.as_float()), Some(1.0), "the preset's value stays");
}

#[test]
fn a_gain_target_drives_the_track_s_strip() {
    let mut b = with_policy(r#"{ "name": "regtest_gain_policy", "domain": "control", "abi": 2,
        "ports": [{ "name": "level", "dir": "out", "type": "control", "path": "drone:@gain", "mode": "mul" }] }"#);
    b.rig.tracks[1].gain = 0.8;
    let p = b.play(None, None).unwrap();
    let node = p.nodes.iter().find(|n| n.id == "policy").unwrap();
    assert_eq!(node.params.get("@gain").and_then(|t| t.get("drone")).and_then(|v| v.as_str()), Some("mix1.gain2"));
    let g = Graph::build(&Scene::parse(&p.scene).unwrap()).unwrap();
    let pi = g.nodes.iter().position(|n| n.id == "policy").unwrap();
    let l = g.links.iter().find(|l| l.from.0 == pi).expect("the strip is bound");
    assert_eq!((g.nodes[l.to.0].id.as_str(), g.nodes[l.to.0].kind.inputs()[l.to.1].name), ("mix1", "gain2"));
    assert_eq!((l.scale, l.offset), (0.8, 0.0), "mul: the track's gain × the output");
    assert_eq!(g.holds.iter().find(|h| h.0 .0 == pi).map(|h| h.1), Some(1.0));
    let face = b.rig.describe(&b.library()).unwrap();
    assert_eq!(face.policy.unwrap().targets[0].resolved, To::Strip { input: "mix1.gain2".into() });
    // a muted track has no strip
    b.rig.tracks[1].mute = true;
    assert!(b.play(None, None).unwrap_err().to_string().contains("the track is muted"));
}

/// `policy::strip` is the chain flatten builds: every played track's out reaches the strip it names
#[test]
fn the_strip_of_each_track_is_where_flatten_sums_it() {
    let mut b = declarative();
    b.rig.moods.clear();
    b.rig.sticks = None;
    b.rig.tracks = (0..9).map(|i| Track { id: format!("d{i}"), instrument: "drone".into(), preset: None, gain: 1.0, pan: 0.0, mute: false }).collect();
    let p = b.play(None, None).unwrap();
    for i in 0..9 {
        let (n, k) = policy::strip(i);
        assert!(p.links.iter().any(|l| l.from == format!("d{i}_mix.out") && l.to == format!("mix{n}.in{k}")), "d{i} → mix{n}.in{k}");
    }
}

#[test]
fn routes_reach_the_policy_both_ways() {
    let mut b = with_policy(r#"{ "name": "regtest_cable_policy", "domain": "control", "abi": 2,
        "ports": [{ "name": "push", "dir": "in", "type": "control" }, { "name": "swell", "dir": "out", "type": "control" }],
        "params": [{ "name": "heat", "min": 0.0, "max": 1.0, "default": 0.0 }] }"#);
    b.instruments.push(serde_json::from_value(serde_json::json!({ "id": "sway", "kind": "pad", "out": "o", "node": [{ "id": "o", "kind": "osc" }, { "id": "l", "kind": "lfo" }] })).unwrap());
    b.rig.tracks.push(Track { id: "sway".into(), instrument: "sway".into(), preset: None, gain: 1.0, pan: 0.0, mute: false });
    b.rig.routes = vec![
        Route { from: "sway:l".into(), to: "policy:push".into(), scale: 1.0, offset: 0.0 },
        Route { from: "policy:swell".into(), to: "being:glow".into(), scale: 0.5, offset: 0.0 },
    ];
    b.rig.policy.as_mut().unwrap().values.insert("heat".into(), 0.3);
    b.rig.moods.insert("happy".into(), Variant { presets: BTreeMap::new(), values: BTreeMap::from([("policy:heat".into(), 0.9)]) });
    let p = b.play(None, Some("happy")).unwrap();
    assert!(p.links.iter().any(|l| l.from == "sway_l.out" && l.to == "policy.push"));
    assert!(p.links.iter().any(|l| l.from == "policy.swell" && l.to == "space.glow" && l.scale == 0.5));
    let node = p.nodes.iter().find(|n| n.id == "policy").unwrap();
    assert_eq!(node.params.get("heat").and_then(|v| v.as_float()), Some(0.9_f32 as f64), "the mood's policy:heat over the policy's own");
    assert_eq!(node.params.get("mood").and_then(|v| v.as_str()), Some("happy"));
    assert_eq!(b.play(None, None).unwrap().knobs.iter().find(|k| k.path == "policy.heat").map(|k| k.value), Some(0.3));
    // a port the module lacks is the graph's error
    b.rig.routes[0].to = "policy:pull".into();
    assert!(format!("{:#}", b.play(None, None).unwrap_err()).contains("has no input 'pull'"));
}

#[test]
fn the_core_refuses_a_policy_that_does_not_resolve() {
    let refuse = |json: &str, edit: &dyn Fn(&mut Bundle), says: &str| {
        let mut b = with_policy(json);
        edit(&mut b);
        let e = format!("{:#}", b.rig.check(&b.library()).unwrap_err());
        assert!(e.contains(says), "{says:?} in {e:?}");
    };
    let keep = |_: &mut Bundle| {};
    let swap = |from: &'static str, to: &'static str| CATPOPE_RIG.replace(from, to);
    refuse(CATPOPE_RIG, &|b| b.modules.clear(), "no module 'catpope_rig' for its policy");
    refuse(&swap(r#""domain": "control""#, r#""domain": "audio""#), &keep, "runs in the Audio domain – a policy is a control module");
    refuse(&swap(r#""abi": 2"#, r#""abi": 1"#), &keep, "speaks ABI 1 – a policy is built for ABI 2");
    refuse(&swap("drone:drone.gain", "harp:drone.gain"), &keep, "no track 'harp'");
    refuse(&swap("drone:drone.gain", "drone:drone.gian"), &keep, "drone.gian is not a control input");
    refuse(&swap("drone:drone.gain", "drone:@pan"), &keep, "only @preset and @gain are a track's reserved members");
    refuse(&swap("drone:drone.gain", "drone.gain"), &keep, "a target is <track>:<node>.<port>");
    refuse(&swap("\"cathedral_voice.bright\"]", "\"drone.base\"]"), &keep, "preset 'drone.base' is for 'drone'");
    refuse(&swap("\"cathedral_voice.bright\"]", "\"cathedral_voice.gold\"]"), &keep, "unknown preset \"cathedral_voice.gold\"");
    refuse(&swap(r#""path": "voice:@preset","#, r#""path": "voice:@preset", "mode": "add","#), &keep, "a preset is chosen (mode set), not add");
    refuse(&swap("drone:drone.gain", "voice:voice.index"), &keep, "the output 'voice_index' drives it too");
    refuse(CATPOPE_RIG, &|b| b.rig.tracks[0].id = "policy".into(), "the track id 'policy' is reserved");
    refuse(CATPOPE_RIG, &|b| _ = b.rig.policy.as_mut().unwrap().values.insert("heat".into(), 2.0), "the policy's heat = 2 is outside 0 … 1");
    refuse(CATPOPE_RIG, &|b| _ = b.rig.policy.as_mut().unwrap().values.insert("cold".into(), 0.5), "has no parameter 'cold'");
    refuse(CATPOPE_RIG, &|b| b.rig.routes.push(Route { from: "drone:drone.out".into(), to: "nobody:x".into(), scale: 1.0, offset: 0.0 }), "no track 'nobody'");
    // without a policy, `policy:` is no scope
    let mut plain = declarative();
    plain.rig.routes.push(Route { from: "policy:swell".into(), to: "being:glow".into(), scale: 1.0, offset: 0.0 });
    assert!(plain.rig.check(&plain.library()).unwrap_err().to_string().contains("no track 'policy'"));
}

#[test]
fn describe_lists_the_children_their_inputs_and_the_policy_s_targets() {
    let b = code_catpope();
    let face = b.rig.describe(&b.library()).unwrap();
    assert_eq!(face.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["voice", "drone"]);
    let voice = &face.tracks[0];
    let index = voice.addressable.iter().find(|a| a.path == "voice:voice.index").expect("an addressable input");
    assert_eq!(index.input, "voice_voice.index");
    assert_eq!(index.driven.as_ref().map(|d| (d.output.as_str(), d.mode)), Some(("policy.voice_index", "set")));
    let ratio = voice.addressable.iter().find(|a| a.path == "voice:voice.ratio").unwrap();
    assert!(ratio.driven.is_none() && ratio.max > ratio.min);
    assert!(voice.addressable.iter().any(|a| a.path == "voice:melody.spread"), "every node's control inputs");
    let policy = face.policy.expect("the policy");
    assert_eq!(policy.module, "catpope_rig");
    assert_eq!(policy.params.iter().map(|p| (p.name.as_str(), p.value)).collect::<Vec<_>>(), [("heat", 0.0)]);
    let resolved: Vec<&To> = policy.targets.iter().map(|t| &t.resolved).collect();
    assert_eq!(
        resolved,
        [
            &To::Preset { track: "voice".into(), choices: vec!["cathedral_voice.base".into(), "cathedral_voice.low".into(), "cathedral_voice.bright".into()] },
            &To::Input { input: "voice_voice.index".into() },
            &To::Input { input: "drone_drone.gain".into() },
        ]
    );
    // the declarative rig: no policy, its moods
    let d = declarative();
    let face = d.rig.describe(&d.library()).unwrap();
    assert!(face.policy.is_none());
    assert_eq!(face.moods, ["happy", "laughing", "sad", "surprised", "thinking"]);
    assert!(face.tracks.iter().flat_map(|t| &t.addressable).all(|a| a.driven.is_none()));
}

/// #313's validation with the module descriptors in the request (the build service's POST /validate runs this)
#[test]
fn the_validator_checks_a_code_rig_with_its_module_descriptor() {
    use crate::rig::validate::{problems_json, Request};
    let b = code_catpope();
    let request = |modules: Vec<serde_json::Value>| Request {
        kind: "rigs".into(),
        id: b.rig.subject.clone(),
        document: serde_json::to_value(&b.rig).unwrap(),
        instruments: b.instruments.iter().map(|i| serde_json::to_value(i).unwrap()).collect(),
        presets: b.presets.iter().map(|p| serde_json::to_value(p).unwrap()).collect(),
        modules,
    };
    let a = request(vec![serde_json::from_str(CATPOPE_RIG).unwrap()]).answer().unwrap();
    assert!(a.problems.is_empty(), "{:?}", a.problems);
    assert_eq!(a.warnings, ["cathedral_voice.low sets voice.index, which the policy drives (voice:voice.index, set): dropped"]);
    let a = request(vec![]).answer().unwrap();
    assert!(a.problems[0].contains("no module 'catpope_rig'"), "{:?}", a.problems);
    // the wire form: warnings only when there are some (the API's fixture answers stay as they were)
    let json = serde_json::to_string(&request(vec![serde_json::from_str(CATPOPE_RIG).unwrap()])).unwrap();
    let answer: serde_json::Value = serde_json::from_str(&problems_json(&json).unwrap()).unwrap();
    assert_eq!(answer["warnings"].as_array().map(Vec::len), Some(1));
    let plain = declarative();
    let r = Request { kind: "rigs".into(), id: plain.rig.subject.clone(), document: serde_json::to_value(&plain.rig).unwrap(), instruments: plain.instruments.iter().map(|i| serde_json::to_value(i).unwrap()).collect(), presets: plain.presets.iter().map(|p| serde_json::to_value(p).unwrap()).collect(), modules: vec![] };
    let answer: serde_json::Value = serde_json::from_str(&problems_json(&serde_json::to_string(&r).unwrap()).unwrap()).unwrap();
    assert_eq!(answer, serde_json::json!({ "problems": [] }));
}

/// the document forms: a rig's `policy` and a bundle's `[[module]]` read and write in TOML and JSON
#[test]
fn a_code_rig_bundle_round_trips() {
    let b = code_catpope();
    let json = serde_json::to_string(&b).unwrap();
    assert_eq!(Bundle::parse_json(&json).unwrap(), b);
    let text = toml::to_string(&b).unwrap();
    assert_eq!(toml::from_str::<Bundle>(&text).unwrap(), b);
    assert!(serde_json::to_value(&declarative().rig).unwrap().get("policy").is_none(), "a declarative rig writes no policy");
    let e = serde_json::from_value::<Rig>(serde_json::json!({ "subject": "x", "track": [], "policy": { "module": "m", "sede": 3 } })).unwrap_err();
    assert!(e.to_string().contains("unknown field `sede`"), "{e}");
}
