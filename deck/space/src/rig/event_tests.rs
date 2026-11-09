//! Event cables, silent tracks and world nodes in a rig (#430, research/EVENTS-VS-PARAMS.md §5): what the rig's flatten and
//! the wiring view answer – the engine's side is deck-engine's module_tests, the real modules deck-native's tests/modules.rs.
use super::instrument::Instrument;
use super::wiring::Wiring;
use super::{Bundle, Library};
use crate::modreg::{self, ModuleDef};
use crate::scene::Scene;
use deck_catalog::Descriptor;

const SEQ: &str = r#"{"abi":2,"name":"ev_seq","version":"1","domain":"control","params":[],"ports":[{"name":"notes","dir":"out","type":"event","names":["music.note"]}]}"#;
const SYNTH: &str = r#"{"abi":2,"name":"ev_synth","version":"1","params":[{"name":"volume","min":0,"max":1,"default":0.5}],"ports":[{"name":"notes","dir":"in","type":"event","names":["music.note"]},{"name":"cues","dir":"in","type":"event","names":["story.cue"]},{"name":"out","dir":"out","type":"audio","channels":2}]}"#;

fn descriptors() -> Vec<Descriptor> {
    [SEQ, SYNTH].iter().map(|j| serde_json::from_str(j).unwrap()).collect()
}

fn register() {
    for d in descriptors() {
        modreg::register(&d.name, &ModuleDef::of(&d));
    }
}

const INSTRUMENTS: &str = r#"
    [[instrument]]
    id = "seq"
    kind = "pad"
    node = [{ id = "seq", kind = "module:ev_seq" }]
    [[instrument]]
    id = "synth"
    kind = "pad"
    out = "synth.out"
    node = [{ id = "synth", kind = "module:ev_synth" }]
"#;

/// a bundle of `rig` (its `[rig]` table's text) over the two instruments
fn bundle(rig: &str, extra: &str) -> Bundle {
    register();
    toml::from_str(&format!("{rig}\n{INSTRUMENTS}\n{extra}")).unwrap()
}

fn lib_of(b: &Bundle) -> Library {
    b.library()
}

fn flat(b: &Bundle) -> Scene {
    b.rig.flatten(&lib_of(b), None, None).unwrap()
}

const TWO_PAIRS: &str = r#"
    [rig]
    subject = "character.two"
    track = [{ id = "a_seq", instrument = "seq" }, { id = "a_synth", instrument = "synth" }, { id = "b_seq", instrument = "seq" }, { id = "b_synth", instrument = "synth" }]
    route = [{ from = "a_seq:seq.notes", to = "a_synth:synth.notes" }, { from = "b_seq:seq.notes", to = "b_synth:synth.notes" }]
"#;

fn event_cables(w: &Wiring) -> Vec<String> {
    w.links.iter().filter(|l| l.ty == "event").map(|l| format!("{}.{} → {}.{}", w.nodes[l.from].id, l.out, w.nodes[l.to].id, l.input)).collect()
}

#[test]
fn two_pairs_are_two_cables_and_the_ports_are_events() {
    let b = bundle(TWO_PAIRS, "");
    let w = b.play(None, None).unwrap().wiring().unwrap();
    assert_eq!(event_cables(&w), ["a_seq_seq.notes → a_synth_synth.notes", "b_seq_seq.notes → b_synth_synth.notes"]);
    let synth = w.nodes.iter().find(|n| n.id == "a_synth_synth").unwrap();
    assert!(synth.inputs.iter().any(|p| p.name == "notes" && p.ty == "event"), "{:?}", synth.inputs);
    let seq = w.nodes.iter().find(|n| n.id == "a_seq_seq").unwrap();
    assert_eq!(seq.outputs.iter().map(|p| (p.name.as_str(), p.ty)).collect::<Vec<_>>(), [("notes", "event")]);
    for l in w.links.iter().filter(|l| l.ty == "event") {
        assert!(w.nodes[l.from].outputs.iter().any(|p| p.name == l.out && p.ty == l.ty));
        assert!(w.nodes[l.to].inputs.iter().any(|p| p.name == l.input && p.ty == l.ty));
    }
}

/// #433: the rig's routes are the rig level's cables (home: the route), and a track level shows the one into it as a
/// stub named by the other end's rig-level leaf
#[test]
fn routes_are_rig_level_cables_and_track_level_stubs() {
    use super::wiring::Home;
    let p = bundle(TWO_PAIRS, "").play(None, None).unwrap();
    let rig = p.wire_view("").unwrap();
    assert_eq!(rig.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["a_seq", "a_synth", "b_seq", "b_synth", "being"]);
    let cables: Vec<(&str, &str, &str, &str)> = rig.links.iter().map(|l| (rig.nodes[l.from].id.as_str(), l.out.as_str(), rig.nodes[l.to].id.as_str(), l.input.as_str())).collect();
    assert_eq!(cables, [("a_seq", "seq.notes", "a_synth", "synth.notes"), ("b_seq", "seq.notes", "b_synth", "synth.notes")]);
    assert_eq!(rig.links.iter().map(|l| (&l.home, l.ty, l.readonly)).collect::<Vec<_>>(), [(&Home::Route { index: 0 }, "event", None), (&Home::Route { index: 1 }, "event", None)]);
    assert!(rig.nodes[0].outputs.iter().any(|p| p.name == "seq.notes" && p.ty == "event"));
    let track = p.wire_view("a_synth").unwrap();
    assert_eq!(track.levels, ["rig", "track:a_synth"]);
    let stubs: Vec<(&str, &str, bool, &str, &Home)> = track.stubs.iter().map(|s| (track.nodes[s.node].id.as_str(), s.port.as_str(), s.input, s.other.as_str(), &s.home)).collect();
    // in the scene's link order: the strip into the sum first, the route after it
    assert_eq!(stubs, [("synth", "out", false, "sum", &Home::Compiled), ("synth", "notes", true, "a_seq:seq.notes", &Home::Route { index: 0 })]);
    let module = p.wire_view("a_synth:synth").unwrap();
    assert_eq!(module.stubs.iter().map(|s| s.other.as_str()).collect::<Vec<_>>(), ["sum", "a_seq:seq.notes"], "another track's end by its rig-level leaf");
}

#[test]
fn a_silent_track_has_no_strip_and_no_outlet() {
    let b = bundle(TWO_PAIRS, "");
    let s = flat(&b);
    assert!(s.links.iter().all(|l| !(l.from.starts_with("a_seq_") || l.from.starts_with("b_seq_")) || l.to.ends_with(".notes")));
    let strips: Vec<&str> = s.links.iter().filter(|l| l.to.starts_with("mix")).map(|l| l.from.as_str()).collect();
    assert_eq!(strips, ["a_synth_synth.out", "b_synth_synth.out"], "the two synths share the strips, no gap for the silent tracks");
}

#[test]
fn a_rig_of_only_silent_tracks_flattens_with_nothing_to_sum() {
    let b = bundle("[rig]\nsubject = \"character.quiet\"\ntrack = [{ id = \"s\", instrument = \"seq\" }]", "");
    let s = flat(&b);
    assert!(s.nodes.iter().any(|n| n.id == "out") && s.nodes.iter().all(|n| !n.id.starts_with("mix")));
}

#[test]
fn a_cable_between_event_and_non_event_is_refused_with_a_sentence_at_the_port() {
    for (from, to, says) in [("a_seq:seq.notes", "a_synth:synth.volume", "'notes' is an event port and 'volume' is a control value"), ("a_synth:synth.out", "b_synth:synth.notes", "'out' is audio and 'notes' is an event port")] {
        let mut b = bundle(TWO_PAIRS, "");
        b.rig.routes[0].from = from.into();
        b.rig.routes[0].to = to.into();
        let e = format!("{:#}", b.rig.flatten(&lib_of(&b), None, None).unwrap_err());
        assert!(e.contains(says) && e.contains("an event cable joins two event ports"), "{e}");
    }
}

#[test]
fn an_event_cable_with_no_name_in_common_or_with_a_scale_is_refused() {
    let mut b = bundle(TWO_PAIRS, "");
    b.rig.routes[0].to = "a_synth:synth.cues".into();
    assert!(format!("{:#}", b.rig.flatten(&lib_of(&b), None, None).unwrap_err()).contains("it sends music.note and the input takes story.cue"));
    let mut b = bundle(TWO_PAIRS, "");
    b.rig.routes[0].scale = 0.5;
    assert!(format!("{:#}", b.rig.flatten(&lib_of(&b), None, None).unwrap_err()).contains("no scale or offset"));
}

#[test]
fn a_silent_instrument_must_have_something_to_send() {
    register();
    let i: Instrument = toml::from_str("id = \"quiet\"\nkind = \"pad\"\nnode = [{ id = \"o\", kind = \"osc\" }]").unwrap();
    assert!(i.check().unwrap_err().to_string().contains("nothing to send"));
    let i: Instrument = toml::from_str("id = \"ok\"\nkind = \"pad\"\nnode = [{ id = \"seq\", kind = \"module:ev_seq\" }]").unwrap();
    i.check().unwrap();
}

#[test]
fn a_world_node_on_a_silent_track_is_cabled_like_a_module_and_shows_its_filter() {
    let b = bundle(
        r#"
        [rig]
        subject = "character.keys"
        track = [{ id = "keys", instrument = "keys" }, { id = "synth", instrument = "synth" }]
        route = [{ from = "keys:midi.events", to = "synth:synth.notes" }]
        "#,
        r#"
        [[instrument]]
        id = "keys"
        kind = "pad"
        node = [{ id = "midi", kind = "world", source = "midi", listen = ["midi.note"] }]
        "#,
    );
    let w = b.play(None, None).unwrap().wiring().unwrap();
    assert_eq!(event_cables(&w), ["keys_midi.events → synth_synth.notes"]);
    let world = w.nodes.iter().find(|n| n.id == "keys_midi").unwrap();
    assert_eq!(world.kind, "world");
    let shown = world.world.as_ref().expect("the world node's source and filter");
    assert_eq!((shown.source, shown.listen.as_slice()), ("midi", &["midi.note".to_string()][..]));
    assert!(w.nodes.iter().all(|n| n.kind == "world" || n.world.is_none()));
}

#[test]
fn a_bad_world_node_is_refused_when_the_rig_is_played() {
    let b = bundle(
        "[rig]\nsubject = \"character.bad\"\ntrack = [{ id = \"k\", instrument = \"k\" }]",
        "[[instrument]]\nid = \"k\"\nkind = \"pad\"\nnode = [{ id = \"w\", kind = \"world\", source = \"carrier_bag\", listen = [\"x.y\"] }]",
    );
    let e = format!("{:#}", b.rig.flatten(&lib_of(&b), None, None).unwrap_err());
    assert!(e.contains("unknown source 'carrier_bag'"), "{e}");
}

/// #432: a far world node names another place (here a deck): the rig builds and plays, the node is drawn with its
/// address, the wiring and the validator list that it plays only while that deck is in the session (a warning, not a
/// refusal; the link carries the events, far.rs #442). Naming the rig itself is a cable's job.
#[test]
fn a_far_world_node_is_accepted_silent_and_listed() {
    let b = bundle(
        r#"
        [rig]
        subject = "character.keys"
        track = [{ id = "far", instrument = "far" }, { id = "synth", instrument = "synth" }]
        route = [{ from = "far:in.events", to = "synth:synth.notes" }]
        "#,
        r#"
        [[instrument]]
        id = "far"
        kind = "pad"
        node = [{ id = "in", kind = "world", source = "far", address = "/deck:left/scene:nave/character:catpope/analog:seq.notes", listen = ["music.note"] }]
        "#,
    );
    let w = b.play(None, None).unwrap().wiring().unwrap();
    assert_eq!(event_cables(&w), ["far_in.events → synth_synth.notes"]);
    let world = w.nodes.iter().find(|n| n.id == "far_in").unwrap().world.as_ref().unwrap();
    assert_eq!((world.source, world.address.as_deref()), ("far", Some("/deck:left/scene:nave/character:catpope/analog:seq.notes")));
    assert_eq!(w.problems.len(), 1, "{:?}", w.problems);
    assert_eq!(w.problems[0].at, "world node 'far_in'");
    assert!(w.problems[0].says.contains("over the relay") && w.problems[0].says.contains("#442"), "{}", w.problems[0]);
    // the validator: no problem, one warning naming the node and its track
    let request = super::validate::Request {
        kind: "rigs".into(),
        id: b.rig.subject.clone(),
        document: serde_json::to_value(&b.rig).unwrap(),
        instruments: b.instruments.iter().map(|i| serde_json::to_value(i).unwrap()).collect(),
        presets: vec![],
        modules: [SEQ, SYNTH].iter().map(|j| serde_json::from_str(j).unwrap()).collect(),
    };
    let a = request.answer().unwrap();
    assert_eq!(a.problems, Vec::<String>::new());
    assert_eq!(a.warnings.len(), 1, "{:?}", a.warnings);
    assert!(a.warnings[0].contains("world node 'in' (track far)") && a.warnings[0].contains("over the relay"), "{:?}", a.warnings);
    // the rig's own place is the rig itself: inside it a cable joins the ports
    let mut own = b.clone();
    own.instruments[2].nodes[0].params.insert("address".into(), toml::Value::String("character:keys/synth:synth.notes".into()));
    let e = format!("{:#}", own.rig.check(&lib_of(&own)).unwrap_err());
    assert!(e.contains("world node 'in' (track far)") && e.contains("names this rig itself"), "{e}");
}

/// an instrument without `out` is a document like any, in JSON too (#430)
#[test]
fn an_instrument_without_out_is_a_document_like_any() {
    let i: Instrument = serde_json::from_str(r#"{"id":"q","kind":"pad","node":[{"id":"s","kind":"module:ev_seq"}]}"#).unwrap();
    assert_eq!(i.out, None);
    assert!(!serde_json::to_string(&i).unwrap().contains("\"out\""), "no `out` key");
}
