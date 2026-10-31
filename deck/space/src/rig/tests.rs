use super::*;
use crate::graph::Graph;
use serde::Deserialize;

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
route = [{ from = "voice:sway.out", to = "being:glow" }, { from = "air:hiss.out", to = "master:in", scale = 0.3 }]
[rig.personas.vespers]
values = { "air:hiss.gain" = 0.1 }
[rig.moods.sad]
presets = { voice = "choir.sad" }
values = { "air:hiss.cutoff" = 400.0 }
[rig.moods.happy]
presets = { voice = "choir.bright" }
"#;

#[derive(Deserialize)]
struct Set {
    instrument: Vec<Instrument>,
    preset: Vec<Preset>,
    rig: Rig,
}

fn set() -> (Library, Rig) {
    let s: Set = toml::from_str(SET).unwrap();
    (Library::new(s.instrument, s.preset), s.rig)
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
