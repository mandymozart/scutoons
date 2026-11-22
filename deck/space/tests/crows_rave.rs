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
