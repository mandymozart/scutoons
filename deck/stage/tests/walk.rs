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
