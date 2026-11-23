//! The three crows rave's stage (#794): the document deck/headless/documents/crows-rave.json – four holders on a row
//! (crow-wolf, crow-lizard, crow-fox in their 3D look, the drum machine's empty holder beside the fox) over the scene's
//! nodes – resolved by the core into the sandbox deck/native/scenes/sandbox/crows-rave.toml. The audio graph stays the
//! scene's (deck-native tests/crows_rave.rs plays it); the crows' own rigs come with #346.
use deck_space::cast::{drawing, form, stage_3d, Drawing, Form, Who};
use deck_space::scene::Scene;
use deck_space::scene_document::SceneDocument;
use deck_space::scene_resolve::{core_rigs, no_sets, resolve, Resolved};
use std::collections::BTreeMap;

const DOCUMENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../headless/documents/crows-rave.json");
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../native/scenes/sandbox/crows-rave.toml");

const HEAD: &str = "\
# The three crows rave (#747, research/audio-nodes-foundation.md §4): MIDI input 1, channel 1 plays three crows, each an arp
# (up, 1/8, two octaves) into analog's `hoover-dissonant` – the second a minor third up, the third a tritone up: a diminished
# triad on every eighth at 160 BPM. The drum machine stands next to the fox crow: drumseq's groove into four analog lanes.
# The analog values are analog.preset.json's presets written out (deck-native tests/crows_rave.rs holds them equal) until
# a node can name a module preset (#346).
# On the stage (#794): four holders on a row – crow-wolf, crow-lizard, crow-fox in their 3D look, the drum machine's empty
# holder beside the fox. Until each crow plays its own rig (#346) the default character rig stands in, its voice at 0.
# Written by deck-space's tests/crows_rave.rs from deck/headless/documents/crows-rave.json – never by hand:
#   UPDATE_FIXTURES=1 cargo test -p deck-space --test crows_rave
# Needs the `modules` build with arp, analog, drumseq, delay and reverb in DECK_MODULES:
#   cargo run --release -p deck-native --features modules -- --dev --scene crows-rave
";

fn update() -> bool {
    std::env::var_os("UPDATE_FIXTURES").is_some()
}

/// the document, its text the canonical one (`UPDATE_FIXTURES=1` writes it)
fn document() -> SceneDocument {
    let stored = std::fs::read_to_string(DOCUMENT).unwrap().replace('\r', "");
    let doc = SceneDocument::parse_json(&stored).unwrap();
    let canonical = doc.to_json().unwrap() + "\n";
    if update() {
        std::fs::write(DOCUMENT, &canonical).unwrap();
    } else {
        assert_eq!(stored, canonical, "crows-rave.json is not canonical: UPDATE_FIXTURES=1 cargo test -p deck-space --test crows_rave");
    }
    assert_eq!(doc.check(), Vec::<String>::new());
    doc
}

/// the scene's module kinds, as a deck that loaded them knows them
fn register_modules() {
    use deck_module::Describe;
    for json in [arp_module::Arp::DESCRIPTOR, analog_module::Analog::DESCRIPTOR, drumseq_module::DrumSeq::DESCRIPTOR, delay_module::Delay::DESCRIPTOR, reverb_module::Reverb::DESCRIPTOR] {
        let d: deck_catalog::Descriptor = serde_json::from_str(json).unwrap();
        deck_space::modreg::register(&d.name, &deck_space::modreg::ModuleDef::of(&d));
    }
}

/// as a deck resolves it: its core rigs (a crow without a rig of its own plays the default character rig)
fn resolved() -> Resolved {
    register_modules();
    resolve(&document(), &BTreeMap::new(), false, &core_rigs, &no_sets).unwrap()
}

#[test]
fn the_sandbox_is_the_core_s_resolve_of_the_document() {
    let scene = resolved().scene;
    let text = format!("{HEAD}\n{}", toml::to_string(&scene).unwrap());
    if update() {
        std::fs::write(SANDBOX, &text).unwrap();
        return;
    }
    assert_eq!(std::fs::read_to_string(SANDBOX).unwrap().replace('\r', ""), text, "crows-rave.toml is stale: UPDATE_FIXTURES=1 cargo test -p deck-space --test crows_rave");
    assert_eq!(Scene::parse(&text).unwrap(), scene, "what the decks read is what the core resolved");
}

/// three different crows, left to right, each in its 3D look – drawn as its model once scene-content names one – and the
/// drum machine's holder empty beside the fox: no member, no look
#[test]
fn three_crows_stand_in_a_row_in_3d_beside_the_drum_machine() {
    let r = resolved();
    let looks: Vec<(&str, &str, f64)> = r
        .scene
        .nodes
        .iter()
        .filter(|n| n.kind == "look")
        .map(|n| (n.params["holder"].as_str().unwrap(), n.params["who"].as_str().unwrap(), n.params["at"][0].as_float().unwrap()))
        .collect();
    assert_eq!(looks, [("wolf", "character:crow-wolf", -2.25), ("lizard", "character:crow-lizard", -0.75), ("fox", "character:crow-fox", 0.75)]);
    let s3 = stage_3d(&r.scene);
    for n in r.scene.nodes.iter().filter(|n| n.kind == "look") {
        let chosen = n.params.get("form").and_then(|v| v.as_str()).and_then(Form::parse);
        let f = form(&Who::parse(n.params["who"].as_str().unwrap()).unwrap(), s3, chosen, None);
        assert_eq!((f, drawing(f, true), drawing(f, false)), (Form::Look, Drawing::Model, Drawing::Being), "{}", n.id);
    }
    let doc = document();
    let drums = doc.holders.iter().find(|h| h.id == "drum-machine").unwrap();
    assert!(drums.is_placeholder() && drums.at.unwrap()[0] > 0.75, "the drum machine's holder, right of the fox");
    assert!(r.cast.iter().all(|m| m.holder != "drum-machine"), "empty: no member");
}

/// each crow is drawn as its own model: scene-content's answer (server SceneContentTest writes it) names crow-wolf's and
/// crow-fox's from the persona, crow-lizard's from its look – the deck fetches each as `Want::Model` and draws the model
#[test]
fn each_crow_is_drawn_as_its_own_model_from_its_persona_or_its_look() {
    use deck_space::push::{Content, Want};
    let answer = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/scene-content-crows.json")).unwrap();
    let content = Content::parse(&answer).unwrap();
    let r = resolved();
    let s3 = stage_3d(&r.scene);
    let mut models = Vec::new();
    for n in r.scene.nodes.iter().filter(|n| n.kind == "look") {
        let who = Who::parse(n.params["who"].as_str().unwrap()).unwrap();
        let chosen = n.params.get("form").and_then(|v| v.as_str()).and_then(Form::parse);
        let f = form(&who, s3, chosen, content.look_form(who.character()));
        let by = content.model_of(who.character());
        assert_eq!((drawing(f, by.is_some()), by), (Drawing::Model, Some(who.character())), "{} drawn as its own model", who.character());
        let file = content.file(&Want::Model { character: who.character().into() }).unwrap().unwrap();
        models.push(file.path.clone());
    }
    assert_eq!(models, ["characters/crow-wolf/crow-wolf-regular.glb", "characters/crow-lizard/crow-lizard.glb", "characters/crow-fox/crow-fox-regular.glb"]);
}

/// the stage adds no sound: the default rig's voice stands in at 0 for each crow, and every node and cable of the scene
/// is the document's as written
#[test]
fn the_holders_leave_the_audio_graph_as_it_is() {
    let doc = document();
    let r = resolved();
    for crow in ["wolf", "lizard", "fox"] {
        let voice = r.scene.nodes.iter().find(|n| n.id == format!("{crow}_voice_voice")).unwrap_or_else(|| panic!("{crow}: the default voice"));
        assert_eq!(voice.params["gain"].as_float(), Some(0.0), "{crow}: silent");
    }
    for node in &doc.nodes {
        let got = r.scene.nodes.iter().find(|n| n.id == node.id).unwrap();
        for (k, v) in &node.params {
            assert_eq!(got.params.get(k), Some(v), "{}.{k}", node.id);
        }
    }
    for l in &doc.links {
        assert!(r.scene.links.contains(l), "{} → {}", l.from, l.to);
    }
    assert_eq!(r.scene.nodes.iter().filter(|n| n.kind == "audio_out").count(), 1);
}

const DRUM_MACHINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/drum-machine.json");

/// the drum machine as a rig (server/tests/fixtures/rigs/drum-machine.json): the sequencer and the four voices, its body
/// the Takeyari node
fn drum_machine() -> deck_space::rig::Bundle {
    serde_json::from_str(&std::fs::read_to_string(DRUM_MACHINE).unwrap()).unwrap()
}

/// a body-part rig: `item.drum-machine`, its tracks the drumseq and crows-rave's four drum voices with their values as
/// written, each lane cabled to its voice, its body the Takeyari exhaust's model – the core checks it as the catalog does
#[test]
fn the_drum_machine_is_a_rig_with_the_takeyari_body() {
    register_modules();
    let b = drum_machine();
    b.rig.check(&b.library()).unwrap();
    assert_eq!(b.rig.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["seq", "kick", "snare", "hat_closed", "hat_open"]);
    assert_eq!(b.rig.body.as_ref().map(|b| b.model.as_str()), Some("characters/takeyari-drum/takeyari-drum-regular.glb"));
    let scene = b.rig.flatten(&b.library(), None, None).unwrap();
    let doc = document();
    for (track, node) in [("seq", "drumseq"), ("kick", "drum-kick"), ("snare", "drum-snare"), ("hat_closed", "drum-hat-closed"), ("hat_open", "drum-hat-open")] {
        let inner = if track == "seq" { "seq" } else { "voice" };
        let got = scene.nodes.iter().find(|n| n.id == format!("{track}_{inner}")).unwrap_or_else(|| panic!("{track}"));
        let want = doc.nodes.iter().find(|n| n.id == node).unwrap();
        assert_eq!((&got.kind, &got.params), (&want.kind, &want.params), "{track} plays crows-rave's {node}");
    }
    for (lane, voice) in [(1, "kick"), (2, "snare"), (3, "hat_closed"), (4, "hat_open")] {
        assert!(scene.links.iter().any(|l| l.from == format!("seq_seq.lane{lane}") && l.to == format!("{voice}_voice.notes")), "lane {lane} → {voice}");
    }
}

/// scene-content's answer for `items=drum-machine` (server SceneContentRigsTest writes it): the rig a deck places is the
/// fixture's, its body served beside it as a file with its size
#[test]
fn scene_content_serves_the_drum_machine_with_its_body() {
    let answer = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content-drum-machine.json")).unwrap();
    let rigs = deck_space::rig::content::SceneRigs::parse_json(&answer).unwrap();
    let served = rigs.served("item.drum-machine").expect("the item's rig");
    assert_eq!((&served.rig, &served.instruments), (&drum_machine().rig, &drum_machine().instruments));
    let v: serde_json::Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(v["items"]["drum-machine"]["body"]["path"], served.rig.body.as_ref().unwrap().model.as_str());
    assert!(v["items"]["drum-machine"]["body"]["size"].is_u64());
}

/// crows-rave's drum-machine holder as a placed rig (`rig: rig:item.drum-machine`, no `fill`): the rig stands at the
/// holder's place as its body – in 3D, drawn as the model once it is here – its tracks under the holder's prefix; a use
/// cannot replace it
#[test]
fn crows_rave_s_drum_machine_holder_places_the_rig() {
    use deck_space::cast::{form_rigged, Rigged};
    use deck_space::scene_document::{Fillers, RigPin};
    register_modules();
    let mut doc = document();
    let h = doc.holders.iter_mut().find(|h| h.id == "drum-machine").unwrap();
    (h.fill, h.label, h.rig) = (None, None, Some(RigPin { rig: "rig:item.drum-machine".into(), version: None }));
    assert_eq!(doc.check(), Vec::<String>::new());
    let b = drum_machine();
    let rigs = |s: &str, v: Option<&str>| if s == "item.drum-machine" { Some(b.clone()) } else { core_rigs(s, v) };
    let r = resolve(&doc, &BTreeMap::new(), false, &rigs, &no_sets).unwrap();
    let look = r.scene.nodes.iter().find(|n| n.id == "drum-machine_look").expect("a look of its own");
    let word = |k: &str| look.params.get(k).and_then(|v| v.as_str());
    assert_eq!((word("who"), word("rig"), word("body")), (Some("item:drum-machine"), Some("item.drum-machine"), Some("characters/takeyari-drum/takeyari-drum-regular.glb")));
    assert_eq!(look.params["at"][0].as_float(), Some(2.25));
    let f = form_rigged(&Who::parse("item:drum-machine").unwrap(), stage_3d(&r.scene), None, None, Rigged { placed: true, body: true });
    assert_eq!((f, drawing(f, true), drawing(f, false)), (Form::Look, Drawing::Model, Drawing::Being));
    assert!(r.scene.links.iter().any(|l| l.from == "drum-machine_seq_seq.lane1" && l.to == "drum-machine_kick_voice.notes"));
    let refused = resolve(&doc, &BTreeMap::from([("drum-machine".to_string(), Fillers::One("item:cowbell".into()))]), false, &rigs, &no_sets).unwrap_err();
    assert_eq!(refused, ["'drum-machine' is a placed rig: nothing outside replaces it"]);
}
