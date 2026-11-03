//! The engine's renders as hashes (`fixtures/native-hashes.json`) – the guard for every engine change (recorded with
//! the native deck's engine before the move of #317, P4: the move kept them, bit for bit; the rewrite of the six fundsp
//! units and the true `fm` re-recorded exactly their scenes; #319's block-rate control the LFO scenes at 1024). One hash per
//! kind scene (and control scene, `CONTROL_CASES`) and block pattern, over the left
//! channel after master and limiter, 2 s at 48 kHz; the stereo scene too. `UPDATE_GOLDENS=1 cargo test -p deck-conformance --test goldens` rewrites the file.
//!
//! Recorded on Windows x86-64. A mismatch is to be read before the fixture is re-recorded, not re-recorded blindly.
mod common;
use common::*;
use deck_native::audio::Build;
use std::collections::BTreeMap;

fn current() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut scenes: Vec<(&str, &str)> = CASES.iter().chain(CONTROL_CASES).map(|c| (c.name, c.toml)).collect();
    scenes.push(("stereo scene", STEREO_TOML));
    for (name, toml) in scenes {
        for &block in &[128usize, 1024] {
            let detect = stage_render(toml, block, FRAMES, Build::detect());
            let baseline = stage_render(toml, block, FRAMES, Build { batch: true, avx2: false });
            assert_eq!(hash(&detect), hash(&baseline), "{name}/{block}: avx2 ≡ baseline");
            map.insert(format!("{name}/{block}"), format!("{:016x}", hash(&detect)));
        }
    }
    map
}

#[test]
fn the_native_deck_renders_what_the_fixture_says() {
    let now = current();
    let path = fixture_path("native-hashes.json");
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        let doc = serde_json::json!({
            "about": "FNV-1a 64 over the f32 bits of deck-engine's left channel (2 s, 48 kHz, master + limiter), per kind scene and block size. #309, deck/conformance/tests/goldens.rs",
            "re_recorded": "#317 (the engine move): the move itself kept every hash; re-recorded were osc saw / square / triangle and noise white / pink (fundsp's six units became deck-core's dsp units), fm, walk -> fm.cv and the stereo scene (true FM, no abs() fold: index x ratio > 1), seq -> osc.cv, click and walk -> fm.cv (std powf / sin / exp became libm: native == wasm bit for bit). lfo sine / saw at the 1024 pattern (the fixed 128-frame quantum: the ramp that glides a changed param spans 128 frames, not 1024; their 128 hashes are unchanged). Unchanged since #309: osc sine, mixer, fft tap and lfo at 128. #319 (the control half at block rate, inside the engine): every hash kept but lfo sine / saw at 1024 – the control half now steps once per 128-frame quantum whatever the caller's block, so their 1024 hashes became their 128 ones; the control scenes (CONTROL_CASES) added.",
            "recorded_on": "windows x86-64",
            "hashes": now,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
        println!("fixture rewritten: {}", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).expect("fixtures/native-hashes.json (UPDATE_GOLDENS=1 writes it)");
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let recorded = doc["hashes"].as_object().expect("hashes");
    assert_eq!(recorded.len(), now.len(), "the same scenes as the fixture");
    let mut wrong = Vec::new();
    for (k, v) in &now {
        if recorded.get(k).and_then(|x| x.as_str()) != Some(v.as_str()) {
            wrong.push(format!("{k}: recorded {:?}, now {v}", recorded.get(k)));
        }
    }
    assert!(wrong.is_empty(), "the native deck's renders changed:\n{}", wrong.join("\n"));
}
