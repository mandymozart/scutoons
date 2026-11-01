use super::*;
use crate::graph::Graph;
use deck_core::batch::BatchGraph;

/// A test set in TOML – how the core set would ship in the image: the cathedral's choir voice (seq → fm, an LFO on its
/// index), and the incense (#210's object sound).
const SET: &str = r#"
[[instrument]]
id = "choir"
kind = "voice"
out = "voice"
node = [
  { id = "seq", kind = "seq", notes = [62, 65, 69], rate = 1.0 },
  { id = "voice", kind = "fm", ratio = 1.5, index = 2.0, gain = 0.16 },
  { id = "sway", kind = "lfo", rate = 0.07, depth = 0.6 },
]
link = [
  { from = "seq.cv", to = "voice.cv" },
  { from = "sway", to = "voice.index", scale = 2.0, offset = 1.0 },
]
expose = [{ path = "voice.ratio", label = { en = "FM ratio" } }, { path = "voice.gain" }, { path = "sway.rate" }]

[[instrument]]
id = "incense"
kind = "texture"
out = "hiss"
node = [{ id = "hiss", kind = "noise", cutoff = 1800.0, gain = 0.0 }]

[[preset]]
id = "choir.base"
instrument = "choir"
tags = ["cathedral"]
values = { "voice.ratio" = 1.5, "voice.gain" = 0.16 }

[[preset]]
id = "choir.bright"
instrument = "choir"
parent = "choir.base"
values = { "voice.ratio" = 3.0 }
mapping = [{ source = "face:jawOpen", target = "voice.gain" }]

[[preset]]
id = "choir.bright.free"
instrument = "choir"
parent = "choir.bright"
unlink = ["voice.index"]
values = { "voice.index" = 5.0 }

[[preset]]
id = "choir.sad"
instrument = "choir"
parent = "choir.base"
values = { "voice.ratio" = 0.5, "sway.rate" = 0.02 }

[[preset]]
id = "incense.base"
instrument = "incense"
values = { "hiss.cutoff" = 900.0, "hiss.gain" = 0.05 }

[rig]
subject = "character.catpope"
track = [
  { id = "voice", instrument = "choir", preset = "choir.base" },
  { id = "air", instrument = "incense", preset = "incense.base", gain = 0.5 },
]
route = [{ from = "voice:sway.out", to = "being:glow", scale = 0.5 }]
[rig.personas.vespers]
values = { "air:hiss.gain" = 0.1 }
[rig.moods.sad]
presets = { voice = "choir.sad" }
values = { "air:hiss.cutoff" = 400.0 }
[rig.moods.happy]
presets = { voice = "choir.bright" }
"#;

fn bundle() -> Bundle {
    toml::from_str(SET).unwrap()
}

fn set() -> (Library, Rig) {
    let b = bundle();
    (b.library(), b.rig)
}

fn node<'g>(g: &'g Graph, id: &str) -> &'g crate::graph::Node {
    g.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn instruments_check_as_graphs() {
    let (lib, _) = set();
    for i in lib.instruments.values() {
        i.check().unwrap();
    }
    let mut bad = lib.instrument("incense").unwrap().clone();
    bad.nodes.push(toml::from_str("id = \"out\"\nkind = \"audio_out\"").unwrap());
    assert!(bad.check().unwrap_err().to_string().contains("belongs to the scene"));
    let mut bad = lib.instrument("choir").unwrap().clone();
    bad.out = "sway".into();
    assert!(bad.check().unwrap_err().to_string().contains("not an audio output"));
    bad.out = "voice".into();
    bad.expose.push(Expose { path: "voice.cv".into(), label: Text::new(), range: None });
    assert!(bad.check().unwrap_err().to_string().contains("not a control input"));
}

#[test]
fn lineage_runs_root_first_and_folds_to_the_leaf() {
    let (lib, _) = set();
    let ids: Vec<&str> = lib.presets.lineage("choir.bright.free").unwrap().iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["choir.base", "choir.bright", "choir.bright.free"]);
    let r = lib.presets.resolve("choir.bright.free").unwrap();
    assert_eq!(r.values["voice.ratio"], 3.0, "the child's value wins");
    assert_eq!(r.values["voice.gain"], 0.16, "the root's value is inherited");
    assert_eq!(r.values["voice.index"], 5.0);
    assert!(r.unlink.contains("voice.index"));
    assert_eq!(r.mappings.len(), 1, "the parent's mapping is inherited");
}

#[test]
fn lineage_refuses_loops_strangers_and_orphans() {
    let (lib, _) = set();
    let mut book = lib.presets.clone();
    book.presets.get_mut("choir.base").unwrap().parent = Some("choir.bright".into());
    assert!(book.lineage("choir.bright").unwrap_err().to_string().contains("loops"));
    let mut book = lib.presets.clone();
    book.presets.get_mut("choir.base").unwrap().parent = Some("incense.base".into());
    assert!(book.lineage("choir.bright").unwrap_err().to_string().contains("another instrument"));
    let mut book = lib.presets.clone();
    book.presets.get_mut("choir.base").unwrap().parent = Some("gone".into());
    assert!(format!("{:#}", book.lineage("choir.bright").unwrap_err()).contains("no preset 'gone'"));
}

#[test]
fn a_preset_patches_the_graph() {
    let (lib, _) = set();
    let choir = lib.instrument("choir").unwrap();
    let patch = choir.patch(&lib.presets.resolve("choir.bright").unwrap()).unwrap();
    let g = patch.graph().unwrap();
    assert_eq!(node(&g, "voice").f32("ratio", 0.0), 3.0);
    assert_eq!(node(&g, "voice").f32("gain", 0.0), 0.16);
    assert_eq!(g.links.len(), 2, "no routing change");
    assert_eq!(patch.mappings[0].source, "face:jawOpen");
    // unlinking the LFO frees the index for a value
    let free = choir.patch(&lib.presets.resolve("choir.bright.free").unwrap()).unwrap();
    assert_eq!(free.links.len(), 1);
    assert_eq!(free.value("voice.index"), Some(5.0));
    // the instrument itself is untouched
    assert_eq!(choir.nodes[1].params["ratio"].as_float(), Some(1.5));
}

#[test]
fn a_preset_cannot_set_a_linked_input_or_a_missing_one() {
    let (lib, _) = set();
    let choir = lib.instrument("choir").unwrap();
    let mut r = Resolved::bare("choir");
    r.values.insert("voice.index".into(), 4.0);
    assert!(choir.patch(&r).unwrap_err().to_string().contains("is linked"));
    let mut r = Resolved::bare("choir");
    r.values.insert("voice.wobble".into(), 1.0);
    assert!(choir.patch(&r).is_err());
    assert!(choir.patch(&Resolved::bare("incense")).unwrap_err().to_string().contains("applied to"));
}

#[test]
fn an_added_link_replaces_the_inputs_link() {
    let (lib, _) = set();
    let choir = lib.instrument("choir").unwrap();
    let mut r = Resolved::bare("choir");
    let l: crate::scene::LinkDef = toml::from_str("from = \"sway\"\nto = \"voice.ratio\"").unwrap();
    r.links.insert("voice.ratio".into(), l);
    r.unlink.insert("voice.index".into());
    let g = choir.patch(&r).unwrap().graph().unwrap();
    assert_eq!(g.links.len(), 2, "seq → cv stays, sway moved from the index to the ratio");
}

#[test]
fn a_mood_switches_presets_and_values() {
    let (lib, rig) = set();
    rig.check(&lib).unwrap();
    let base = rig.play(&lib, None, None).unwrap();
    assert_eq!(base.len(), 2);
    assert_eq!(base[0].preset.as_deref(), Some("choir.base"));
    assert_eq!(base[0].patch.value("voice.ratio"), Some(1.5));
    assert_eq!(base[1].patch.value("hiss.cutoff"), Some(900.0));

    let sad = rig.play(&lib, None, Some("sad")).unwrap();
    assert_eq!(sad[0].preset.as_deref(), Some("choir.sad"));
    assert_eq!(sad[0].patch.value("voice.ratio"), Some(0.5));
    assert_eq!(sad[0].patch.value("voice.gain"), Some(0.16), "folded through choir.base");
    assert_eq!(sad[1].patch.value("hiss.cutoff"), Some(400.0), "the mood's single value");

    // a mood the rig doesn't name plays the defaults
    assert_eq!(rig.play(&lib, None, Some("dizzy")).unwrap(), base);
}

#[test]
fn the_mood_goes_over_the_persona() {
    let (lib, mut rig) = set();
    rig.personas.get_mut("vespers").unwrap().values.insert("air:hiss.cutoff".into(), 2000.0);
    let v = rig.play(&lib, Some("vespers"), None).unwrap();
    assert_eq!(v[1].patch.value("hiss.gain"), Some(0.1));
    assert_eq!(v[1].patch.value("hiss.cutoff"), Some(2000.0));
    let vs = rig.play(&lib, Some("vespers"), Some("sad")).unwrap();
    assert_eq!(vs[1].patch.value("hiss.gain"), Some(0.1), "the persona's value stays");
    assert_eq!(vs[1].patch.value("hiss.cutoff"), Some(400.0), "the mood's wins");
    rig.tracks[1].mute = true;
    assert_eq!(rig.play(&lib, None, None).unwrap().len(), 1, "a muted track isn't built");
}

#[test]
fn a_rig_naming_the_wrong_things_is_refused() {
    let (lib, rig) = set();
    let mut r = rig.clone();
    r.tracks[0].preset = Some("incense.base".into());
    assert!(r.check(&lib).unwrap_err().to_string().contains("is for 'incense'"));
    let mut r = rig.clone();
    r.moods.get_mut("sad").unwrap().presets.insert("drums".into(), "choir.sad".into());
    assert!(format!("{:#}", r.check(&lib).unwrap_err()).contains("no track 'drums'"));
    let mut r = rig.clone();
    r.routes[0].to = "glow".into();
    assert!(r.check(&lib).is_err());
}

#[test]
fn a_derivative_keeps_only_what_changed() {
    let (lib, _) = set();
    let edits = BTreeMap::from([("voice.ratio".to_string(), 3.0), ("voice.gain".to_string(), 0.2)]);
    let child = lib.presets.derive("choir.bright", "choir.mine", Hold::Owned, &edits).unwrap();
    assert_eq!(child.parent.as_deref(), Some("choir.bright"));
    assert_eq!(child.values, BTreeMap::from([("voice.gain".to_string(), 0.2)]));
    let mut book = lib.presets.clone();
    book.presets.insert(child.id.clone(), child);
    assert_eq!(book.resolve("choir.mine").unwrap().values["voice.ratio"], 3.0);
    assert!(lib.presets.derive("choir.bright", "x", Hold::Lent, &edits).unwrap_err().to_string().contains("Lent"));
    assert!(lib.presets.derive("choir.bright", "choir.sad", Hold::Owned, &edits).is_err(), "the id is taken");
}

#[test]
fn rights_default_by_hold() {
    let r = Rights::default();
    assert!(r.allows(Hold::Lent, Right::Tweak) && !r.allows(Hold::Lent, Right::Derive));
    assert!(r.allows(Hold::Core, Right::Derive) && !r.allows(Hold::Core, Right::Copy));
    assert!(r.allows(Hold::Owned, Right::Copy));
}

#[test]
fn documents_round_trip_as_json() {
    let (lib, rig) = set();
    let json = serde_json::to_string(&rig).unwrap();
    assert_eq!(serde_json::from_str::<Rig>(&json).unwrap(), rig);
    for i in lib.instruments.values() {
        assert_eq!(&serde_json::from_str::<Instrument>(&serde_json::to_string(i).unwrap()).unwrap(), i);
    }
    let p = lib.presets.get("choir.bright").unwrap();
    let json = serde_json::to_value(p).unwrap();
    assert_eq!(json["parent"], "choir.base");
    assert!(json.get("unlink").is_none(), "empty lists stay out of the document");
    assert_eq!(&serde_json::from_value::<Preset>(json).unwrap(), p);
}

fn param(s: &crate::scene::Scene, path: &str) -> Option<f64> {
    let (n, p) = crate::scene::split_ref(path);
    s.nodes.iter().find(|x| x.id == n)?.params.get(p)?.as_float()
}

fn has_link(s: &crate::scene::Scene, from: &str, to: &str) -> bool {
    s.links.iter().any(|l| l.from == from && l.to == to)
}

#[test]
fn a_rig_flattens_into_one_scene() {
    let (lib, rig) = set();
    let s = rig.flatten(&lib, None, None).unwrap();
    let g = Graph::build(&s).unwrap();
    for id in ["voice_seq", "voice_voice", "voice_sway", "air_hiss", "mix1", "out", "fft", "space"] {
        assert!(g.nodes.iter().any(|n| n.id == id), "{id}");
    }
    assert!(has_link(&s, "voice_seq.cv", "voice_voice.cv"), "the track's own links, prefixed");
    assert!(has_link(&s, "voice_voice.out", "mix1.in1") && has_link(&s, "air_hiss.out", "mix1.in2"));
    assert_eq!(param(&s, "mix1.gain2"), Some(0.5), "the track's gain is its strip");
    assert!(has_link(&s, "mix1.out", "out.in") && has_link(&s, "mix1.out", "fft.in"));
    let glow = s.links.iter().find(|l| l.to == "space.glow").unwrap();
    assert_eq!((glow.from.as_str(), glow.scale), ("voice_sway.out", 0.5), "being:glow → the space node");
    assert!(has_link(&s, "fft.level", "space.pulse"), "the being breathes with the sum");
    let mut own = rig.clone();
    own.routes[0].to = "being:pulse".into();
    let s2 = own.flatten(&lib, None, None).unwrap();
    assert!(!has_link(&s2, "fft.level", "space.pulse") && has_link(&s2, "voice_sway.out", "space.pulse"), "a route's pulse wins");
    assert_eq!(param(&s, "voice_voice.ratio"), Some(1.5));
    #[cfg(feature = "being")]
    {
        assert_eq!(s.being.character, "catpope");
        assert_eq!(s.being.expose, ["voice_voice.ratio", "voice_voice.gain", "voice_sway.rate"]);
    }
    let sad = rig.flatten(&lib, None, Some("sad")).unwrap();
    assert_eq!(param(&sad, "voice_voice.ratio"), Some(0.5), "the mood's preset");
    assert_eq!(param(&sad, "air_hiss.cutoff"), Some(400.0));
}

#[test]
fn more_than_four_tracks_chain_the_mixers() {
    let (lib, mut rig) = set();
    for i in 0..4 {
        rig.tracks.push(Track { id: format!("air{i}"), instrument: "incense".into(), preset: None, gain: 1.0, pan: 0.0, mute: false });
    }
    let s = rig.flatten(&lib, None, None).unwrap();
    assert!(has_link(&s, "air1_hiss.out", "mix1.in4"), "the first four on mix1");
    assert!(has_link(&s, "mix1.out", "mix2.in1"), "mix1 into the next");
    assert!(has_link(&s, "air2_hiss.out", "mix2.in2") && has_link(&s, "air3_hiss.out", "mix2.in3"));
    assert!(has_link(&s, "mix2.out", "out.in") && !has_link(&s, "mix1.out", "out.in"));
}

#[test]
fn muted_tracks_and_their_routes_stay_out() {
    let (lib, mut rig) = set();
    rig.tracks[0].mute = true;
    let s = rig.flatten(&lib, None, None).unwrap();
    assert!(!s.nodes.iter().any(|n| n.id.starts_with("voice_")), "no voice");
    assert!(!s.links.iter().any(|l| l.to == "space.glow"), "no glow route");
    assert!(has_link(&s, "air_hiss.out", "mix1.in1"));
    rig.tracks[1].mute = true;
    assert!(rig.flatten(&lib, None, None).unwrap_err().to_string().contains("muted"));
}

#[test]
fn routes_go_between_tracks_or_to_the_being_only() {
    let (lib, mut rig) = set();
    rig.routes[0].to = "master:in".into();
    assert!(rig.check(&lib).is_err(), "every track reaches the master through its strip");
    rig.routes[0] = Route { from: "being:glow".into(), to: "air:hiss.gain".into(), scale: 1.0, offset: 0.0 };
    assert!(rig.check(&lib).is_err(), "the being is a target, not a source");
    rig.routes[0] = Route { from: "voice:sway.out".into(), to: "air:hiss.cutoff".into(), scale: 2000.0, offset: 900.0 };
    let s = rig.flatten(&lib, None, None).unwrap();
    assert!(has_link(&s, "voice_sway.out", "air_hiss.cutoff"), "a modulation across tracks");
}

#[test]
fn a_bundle_plays_into_a_scene_and_knobs() {
    let p = bundle().play(None, None).unwrap();
    assert_eq!(p.character, "catpope");
    let paths: Vec<&str> = p.knobs.iter().map(|k| k.path.as_str()).collect();
    assert_eq!(paths, ["voice_voice.ratio", "voice_voice.gain", "voice_sway.rate"]);
    let ratio = &p.knobs[0];
    assert_eq!((ratio.min, ratio.max, ratio.value, ratio.log), (0.25, 8.0, 1.5, true), "params.rs's range, the preset's value");
    assert_eq!(ratio.label["en"], "FM ratio");
    assert_eq!(p.tracks[0].preset.as_deref(), Some("choir.base"));
    // the scene text is a stage scene that builds the same graph
    let scene = crate::scene::Scene::parse(&p.scene).unwrap();
    assert_eq!(scene.nodes, p.nodes);
    assert_eq!(scene.links.len(), p.links.len());
    Graph::build(&scene).unwrap();
    #[cfg(feature = "being")]
    assert_eq!(scene.being.expose, paths);
}

#[test]
fn a_mood_changes_what_the_page_gets() {
    let b = bundle();
    let sad = b.play(None, Some("sad")).unwrap();
    assert_eq!(sad.knobs[0].value, 0.5);
    assert_eq!(sad.knobs[2].value, 0.02, "sway.rate from choir.sad");
    assert_eq!(sad.tracks[0].preset.as_deref(), Some("choir.sad"));
}

#[test]
fn the_wasm_boundary_speaks_json() {
    let json = serde_json::to_string(&bundle()).unwrap();
    let out: serde_json::Value = serde_json::from_str(&Bundle::play_json(&json, "", "happy").unwrap()).unwrap();
    assert_eq!(out["tracks"][0]["preset"], "choir.bright");
    assert_eq!(out["knobs"][0]["value"], 3.0);
    assert!(out["knobs"][1].get("log").is_none(), "a linear knob says nothing");
    assert!(Bundle::play_json("{}", "", "").is_err());
    assert!(Bundle::play_json(&json, "", "dizzy").is_ok(), "an unknown mood plays the defaults");
}

#[test]
fn the_core_rigs_play_in_every_mood() {
    for (subject, _) in bundle::CORE {
        let b = Bundle::core(subject).unwrap().unwrap();
        assert_eq!(b.rig.subject, subject);
        b.rig.check(&b.library()).unwrap();
        for mood in [None, Some("neutral"), Some("thinking"), Some("happy"), Some("laughing"), Some("surprised"), Some("sad")] {
            b.play(None, mood).unwrap_or_else(|e| panic!("{subject} {mood:?}: {e:#}"));
        }
    }
    assert!(Bundle::core("character.nobody").is_none());
    assert_eq!(Bundle::core_json("character.nobody").unwrap(), "");
    let json = Bundle::core_json("character.catpope").unwrap();
    assert_eq!(Bundle::parse_json(&json).unwrap().rig.tracks.len(), 2);
}

#[test]
fn catpope_keeps_the_phone_patchs_values() {
    let p = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap();
    let v = |path: &str| p.knobs.iter().find(|k| k.path == path).unwrap().value;
    assert_eq!((v("voice_voice.ratio"), v("voice_voice.index"), v("voice_voice.gain")), (1.5, 2.0, 0.16));
    assert_eq!((v("drone_drone.freq"), v("drone_drone.gain")), (55.0, 0.1));
    let happy = Bundle::core("character.catpope").unwrap().unwrap().play(None, Some("laughing")).unwrap();
    let hv = |path: &str| happy.knobs.iter().find(|k| k.path == path).unwrap().value;
    assert_eq!((hv("voice_voice.ratio"), hv("voice_voice.index")), (3.0, 4.0), "the preset and the mood's value");
}

#[test]
fn the_browser_engine_plays_what_it_can_and_hands_the_rest_to_the_page() {
    // the test set: the choir (seq → fm, an LFO on the index) + the incense at strip gain 0.5
    let e = bundle().play(None, None).unwrap().engine;
    let def = deck_core::SceneDef::parse(&e.scene).unwrap();
    let ids: Vec<&str> = def.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["voice_voice", "voice_sway"], "fm + lfo; seq, noise and the plumbing are gone");
    assert!(def.links.iter().any(|l| l.from == "voice_sway.out" && l.to == "voice_voice.index" && l.scale == 2.0));
    let pages: Vec<(&str, f32)> = e.page.iter().map(|p| (p.node.id.as_str(), p.gain)).collect();
    assert_eq!(pages, [("voice_seq", 1.0), ("air_hiss", 0.5)]);
    assert!(e.dropped.iter().any(|l| l.from == "voice_seq.cv" && l.to == "voice_voice.cv"), "an audio link the engine can't carry");
    assert!(e.dropped.iter().any(|l| l.to == "space.glow"), "the being's route: the page's");
    assert!(e.aliases.is_empty(), "the voice's strip is unity");
    // it sounds: deck-core's batch evaluator renders catpope's core rig (the test set's voice has no pitch of its own –
    // its seq drives it, and the seq is the page's)
    let core = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap().engine;
    let mut g = BatchGraph::build(&deck_core::SceneDef::parse(&core.scene).unwrap(), 48_000.0).unwrap();
    g.step(0.01);
    let mut out = vec![0.0f32; 480];
    g.render(&mut out);
    g.render(&mut out);
    assert!(out.iter().any(|x| x.abs() > 0.01), "the voice and the drone are heard");
}

#[test]
fn a_strip_gain_becomes_a_mul_the_knob_turns() {
    let (_, mut rig) = set();
    let mut b = bundle();
    rig.tracks[0].gain = 0.5;
    b.rig = rig;
    let e = b.play(None, None).unwrap().engine;
    assert_eq!(e.aliases["voice_voice.gain"], "voice_voice_strip.a");
    let def = deck_core::SceneDef::parse(&e.scene).unwrap();
    let strip = def.nodes.iter().find(|n| n.id == "voice_voice_strip").unwrap();
    let f = |k: &str| strip.params[k].as_float().unwrap() as f32;
    assert_eq!((strip.kind.as_str(), f("a"), f("b")), ("mul", 0.16, 0.5), "the knob's value × the strip");
    assert!(def.links.iter().any(|l| l.from == "voice_voice_strip" && l.to == "voice_voice.gain"));
    // the core drone: its two oscillators through the instrument's own mixer, unity strips – both played, no alias
    let core = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap().engine;
    let def = deck_core::SceneDef::parse(&core.scene).unwrap();
    let ids: Vec<&str> = def.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["voice_voice", "drone_drone", "drone_fifth"]);
    assert!(core.page.is_empty() && core.aliases.is_empty());
}

/// shared/live-deck's tests play the core rigs' answers from this file; `UPDATE_FIXTURES=1 cargo test -p deck-space
/// rig_fixture` writes it, a plain run fails when it is stale (like deck/protocol's `--check`)
#[test]
fn rig_fixture_for_the_browser_deck_is_current() {
    let play = |subject: &str, mood: &str| {
        let json = serde_json::to_string(&Bundle::core(subject).unwrap().unwrap()).unwrap();
        serde_json::from_str::<serde_json::Value>(&Bundle::play_json(&json, "", mood).unwrap()).unwrap()
    };
    let fixture = serde_json::json!({
        "catpope": play("character.catpope", ""),
        "catpope_laughing": play("character.catpope", "laughing"),
        "incense_burner": play("item.incense_burner", ""),
    });
    let text = serde_json::to_string_pretty(&fixture).unwrap() + "\n";
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/rigs.json");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(path).unwrap_or_default();
    assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space rig_fixture");
}
