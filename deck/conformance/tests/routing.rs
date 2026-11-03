//! The routing scenes of P4b (#318) as hashes (`fixtures/routing-hashes.json`): every device channel of the P0 routing graph
//! on eight channels, a 5.1 bed on a six-channel device and folded on two, a cue outlet on 3-4, the feedback loop – one hash
//! per scene and block pattern over all channels interleaved (2 s at 48 kHz). `UPDATE_GOLDENS=1 cargo test -p
//! deck-conformance --test routing` rewrites the file. The stereo scenes' hashes stay in native-hashes.json, unchanged.
//! Also asserted: AVX2 ≡ baseline, the block pattern does not matter, the fold on two channels is BS.775 of the six, the
//! routing graph's channels carry what the plan says (cue = the send at half level, silent where nothing plays).
mod common;
use common::routing::cases;
use common::*;
use deck_engine::Build;
use deck_engine_wasm::run_device;
use std::collections::BTreeMap;

fn channel(all: &[f32], d: usize, c: usize) -> Vec<f32> {
    all.chunks_exact(d).map(|f| f[c]).collect()
}

fn current() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for case in cases() {
        let mut first = None;
        for &block in &PATTERNS {
            let detect = run_device(&case.toml, &case.deck, RATE, block, FRAMES, Build::detect()).expect("renders");
            let baseline = run_device(&case.toml, &case.deck, RATE, block, FRAMES, Build { batch: true, avx2: false }).expect("renders");
            assert_eq!(hash(&detect), hash(&baseline), "{}/{block}: avx2 ≡ baseline", case.name);
            assert_eq!(*first.get_or_insert(hash(&detect)), hash(&detect), "{}: the block cut ({block}) changes the sound", case.name);
            assert!(peak(&detect) > 0.01, "{}: it plays", case.name);
            map.insert(format!("{}/{block}", case.name), format!("{:016x}", hash(&detect)));
        }
    }
    map
}

#[test]
fn the_routing_scenes_render_what_the_fixture_says() {
    let now = current();
    let path = fixture_path("routing-hashes.json");
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        let doc = serde_json::json!({
            "about": "FNV-1a 64 over the f32 bits of deck-engine's device channels, interleaved (2 s, 48 kHz, master + group limiters), per routing scene and block size. #318, deck/conformance/tests/routing.rs",
            "recorded_on": "windows x86-64",
            "hashes": now,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
        println!("fixture rewritten: {}", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).expect("fixtures/routing-hashes.json (UPDATE_GOLDENS=1 writes it)");
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let recorded = doc["hashes"].as_object().expect("hashes");
    assert_eq!(recorded.len(), now.len(), "the same scenes as the fixture");
    let wrong: Vec<String> = now.iter().filter(|(k, v)| recorded.get(*k).and_then(|x| x.as_str()) != Some(v.as_str())).map(|(k, v)| format!("{k}: recorded {:?}, now {v}", recorded.get(k))).collect();
    assert!(wrong.is_empty(), "the routing renders changed:\n{}", wrong.join("\n"));
}

/// the plan's acceptance scenes by their numbers: a 5.1 scene on 6 channels and folded on 2 (BS.775), a cue on 3-4, the P0
/// graph's channels
#[test]
fn the_scenes_route_as_the_plan_says() {
    let all = cases();
    let get = |name: &str| all.iter().find(|c| c.name.starts_with(name)).unwrap_or_else(|| panic!("{name}"));
    let render = |c: &common::routing::RoutingCase| run_device(&c.toml, &c.deck, RATE, 128, FRAMES, Build::detect()).unwrap();
    // 5.1: six channels, each its own pitch; on two channels BS.775's Lo = L + C/√2 + Ls/√2, the LFE dropped
    let (six, two) = (render(get("5.1 on a 6")), render(get("5.1 folded")));
    let ch = |c: usize| channel(&six, 6, c);
    assert!((0..6).all(|c| peak(&ch(c)) > 0.05), "every channel of the bed plays");
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let lo: Vec<f32> = (0..FRAMES).map(|i| (ch(0)[i] * 1.0 + ch(2)[i] * h) + ch(4)[i] * h).collect();
    let ro: Vec<f32> = (0..FRAMES).map(|i| (ch(1)[i] * 1.0 + ch(2)[i] * h) + ch(5)[i] * h).collect();
    assert!(diff(&channel(&two, 2, 0), &lo).exact && diff(&channel(&two, 2, 1), &ro).exact, "the fold is BS.775 of the six channels, bit for bit");
    // a cue outlet on channels 3-4: the quiet main and the cue are each on their own pair
    let cue = render(get("cue outlet"));
    let (m, c) = (channel(&cue, 4, 0), channel(&cue, 4, 2));
    assert!(peak(&m) > 0.1 && peak(&c) > 0.1 && diff(&channel(&cue, 4, 0), &channel(&cue, 4, 1)).exact && diff(&c, &channel(&cue, 4, 3)).exact);
    assert!(diff(&m, &c).max_abs > 0.01, "two different signals");
    // the P0 graph: channels 1-2 the folder + the folded bed, 3-4 the send at half level of bus A (a: 220 Hz, 0.1 -> 0.05),
    // 5-8 the VBAP speakers
    let probe = render(get("routing graph"));
    let level = |c: usize| peak(&channel(&probe, 8, c));
    assert!(level(0) > 0.1 && (level(2) - 0.05).abs() < 0.005 && level(2) == level(3), "main {:.3}, cue {:.4}", level(0), level(2));
    assert!(level(4) + level(5) + level(6) + level(7) > 0.1, "the panned source plays on the speakers");
    // azimuth 100: between the speakers at 45 and 135 -> only two of the four play
    let loud = (4..8).filter(|&c| level(c) > 0.001).count();
    assert_eq!(loud, 2, "VBAP lights the pair around the source");
}
