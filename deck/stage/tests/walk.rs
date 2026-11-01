//! The `walk` node (#236, deck-events in the stage): a seeded melody as seq's cv into an FM voice. Two engines with
//! the same seed render the same samples (two decks in step without sending notes); a spread changes the melody; a
//! spread of 0 holds the base.
#![cfg(feature = "events")]
use deck_stage::audio::{Build, Engine};
use deck_stage::graph::Graph;
use deck_stage::scene::Scene;

fn scene(seed: i64, spread: f32) -> String {
    format!(
        r#"
[[node]]
id = "tempo"
kind = "tempo"
bpm = 96
[[node]]
id = "melody"
kind = "walk"
seed = {seed}
base = 147.0
spread = {spread}
rate = 2.0
[[node]]
id = "voice"
kind = "fm"
ratio = 1.5
index = 2.0
gain = 0.2
[[node]]
id = "fft"
kind = "fft"
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "melody.cv"
to = "voice.cv"
[[link]]
from = "voice"
to = "fft.in"
[[link]]
from = "voice"
to = "out.in"
"#
    )
}

fn render(seed: i64, spread: f32, build: Build) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(&scene(seed, spread)).unwrap()).unwrap();
    let (mut engine, _handles) = Engine::build_with(&graph, 48_000.0, 1.0, build).unwrap();
    let mut all = Vec::new();
    let mut buf = vec![0.0f32; 256 * 2];
    for i in 0..375 {
        // 2 s in 256-frame blocks
        engine.mark(i as u64 * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
        all.extend_from_slice(&buf);
    }
    all
}

#[test]
fn the_same_seed_renders_the_same_melody_and_a_spread_moves_it() {
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let a = render(42, 1.0, build);
        assert!(a.iter().any(|x| x.abs() > 0.01), "the voice plays the walk ({})", build.label());
        assert_eq!(a, render(42, 1.0, build), "two decks, one seed: the same samples ({})", build.label());
        assert_ne!(a, render(7, 1.0, build), "another seed ({})", build.label());
        assert_ne!(render(42, 0.0, build), a, "spread 0 holds the base ({})", build.label());
    }
}

/// #236: the `events` node – a patch: an arp on a walk; its input `b` drives the arp's rate
fn arp_scene(seed: i64, b: Option<f32>) -> String {
    let b = b.map_or(String::new(), |v| format!("b = {v}\n"));
    format!(
        r#"
[[node]]
id = "tempo"
kind = "tempo"
bpm = 96
[[node]]
id = "arp"
kind = "events"
{b}generator = [
  {{ id = "walk", kind = "random_walk", seed = {seed}, rate = 0.5, spread = 0.6 }},
  {{ id = "arp", kind = "arp", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = "updown", rate = 4.0, length = 0.6 }},
]
edge = [{{ from = "walk.notes", to = "arp.notes" }}]
out = "arp.notes"
knobs = ["walk.spread", "arp.rate"]
[[node]]
id = "voice"
kind = "fm"
ratio = 2.0
index = 1.0
gain = 0.2
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "arp.cv"
to = "voice.cv"
[[link]]
from = "voice"
to = "out.in"
"#
    )
}

fn render_text(text: &str, build: Build) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(text).unwrap()).unwrap();
    let (mut engine, _handles) = Engine::build_with(&graph, 48_000.0, 1.0, build).unwrap();
    let mut all = Vec::new();
    let mut buf = vec![0.0f32; 256 * 2];
    for i in 0..375 {
        engine.mark(i as u64 * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
        all.extend_from_slice(&buf);
    }
    all
}

#[test]
fn an_events_node_plays_its_patch_and_its_knobs_drive_it() {
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let a = render_text(&arp_scene(42, None), build);
        assert!(a.iter().any(|x| x.abs() > 0.01), "the arp plays ({})", build.label());
        assert_eq!(a, render_text(&arp_scene(42, None), build), "one seed, the same samples ({})", build.label());
        assert_ne!(a, render_text(&arp_scene(7, None), build), "another seed ({})", build.label());
        assert_ne!(a, render_text(&arp_scene(42, Some(8.0)), build), "knob b: the arp's rate ({})", build.label());
    }
    let bad = arp_scene(1, None).replace("mode = \"updown\"", "mode = \"sideways\"");
    let graph = Graph::build(&Scene::parse(&bad).unwrap()).unwrap();
    let err = Engine::build(&graph, 48_000.0, 1.0).err().expect("a bad patch is refused");
    assert!(format!("{err:#}").contains("unknown mode"), "{err:#}");
}
