//! #322 (plan P9a) accept: **catpope's rig sounds the same on the X1 and the phone.** The native deck's path – the scene
//! `--scene rig:catpope@<mood>` loads (deck-native rig.rs: deck-space's `Deck` plays the core rig), built as the stage builds
//! it (deck-engine) – against the browser deck's: the same `Deck`'s `engine` scene through live-core (the worker's side;
//! tests/wasm.rs adds `live_core.wasm`, wasm32 and wasm32 + simd128) and through the headless host's rig player (scenario
//! 09-rig-player): the same f32 bits, every block pattern, every core rig, four moods. The renders' hashes are
//! fixtures/rig-hashes.json – the guard of the rig path (`UPDATE_GOLDENS=1 cargo test -p deck-conformance --test rig`
//! rewrites it; a mismatch is read before it is re-recorded).
mod common;
use common::*;
use std::collections::BTreeMap;

#[test]
fn the_native_deck_plays_a_rig_as_the_browser_deck_does() {
    let mut now = BTreeMap::new();
    let mut wrong = Vec::new();
    for subject in RIGS {
        for mood in RIG_MOODS {
            let scene = browser_rig_scene(subject, mood);
            for &block in &PATTERNS {
                let native = native_rig_render(subject, mood, block, FRAMES);
                let browser = core_render_device(&scene, 2, block, FRAMES);
                let d = diff(&native, &browser);
                if !d.exact {
                    wrong.push(format!("{subject}@{mood}/{block}: {d:?}"));
                }
                now.insert(format!("{subject}@{mood}/{block}"), format!("{:016x}", hash(&native)));
            }
        }
    }
    assert!(wrong.is_empty(), "the native deck and the browser deck play a rig differently:\n{}", wrong.join("\n"));
    let cat = |m: &str| now[&format!("catpope@{m}/128")].clone();
    assert_ne!(cat("neutral"), cat("laughing"), "a mood changes catpope's sound");
    assert!(peak(&native_rig_render("catpope", "neutral", 128, FRAMES)) > 1e-3, "catpope sounds");
    let path = fixture_path("rig-hashes.json");
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        let doc = serde_json::json!({
            "about": "#322: FNV-1a 64 over the f32 bits of the core rigs played natively (deck-native rig.rs -> deck-engine), both channels interleaved, 2 s at 48 kHz, per rig, mood and block size – the browser's live-core renders the same bits (deck/conformance/tests/rig.rs)",
            "recorded_on": "windows x86-64",
            "hashes": now,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
        println!("fixture rewritten: {}", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).expect("fixtures/rig-hashes.json (UPDATE_GOLDENS=1 writes it)");
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let recorded = doc["hashes"].as_object().expect("hashes");
    assert_eq!(recorded.len(), now.len(), "the same renders as the fixture");
    let changed: Vec<String> = now.iter().filter(|(k, v)| recorded.get(*k).and_then(|x| x.as_str()) != Some(v.as_str())).map(|(k, v)| format!("{k}: recorded {:?}, now {v}", recorded.get(k))).collect();
    assert!(changed.is_empty(), "the rigs' renders changed:\n{}", changed.join("\n"));
}

/// The headless host plays catpope's rig as the browser's worker does (live-core's `Host`, the virtual sink: quanta of 128
/// up to the ear + the output lead, hashed as they come) – the native deck renders the same bytes for as many frames.
#[test]
fn the_headless_rig_player_hears_what_the_native_deck_plays() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../headless/scenarios/09-rig-player.toml");
    let bundle = deck_headless::Bundle { name: "09-rig-player".into(), scenario: std::fs::read_to_string(file).unwrap(), files: Default::default() };
    let report = deck_headless::run(&bundle);
    assert_eq!(report.status, deck_headless::Status::Pass, "{}", report.reason);
    // step 2: `run` 1500 ms after `load rig = character.catpope` (no mood: the ladder's first, neutral)
    let heard = report.steps[1]["out"]["audio"].as_str().expect("the run's audio hash").to_string();
    let frames = (1500usize * 48 + 1024).div_ceil(128) * 128; // the ear at 1.5 s + the sink's lead, in whole quanta
    let native = native_rig_render("catpope", "neutral", 128, frames);
    assert_eq!(format!("{:016x}", hash(&native)), heard, "the native deck and the headless host play catpope alike");
}
