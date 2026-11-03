//! The stereo pool's cost, for the 5 % bar of the channel change (#318, plan 3.8 P0d): the P0 base scene (osc, fm, osc, fm →
//! mixer → audio_out → master → limiter) rendered for 10 s in 128-frame blocks, median of seven runs. Ignored by default (timing
//! is not a CI assertion): `cargo test -p deck-engine --test bench -- --ignored --nocapture`.
use deck_engine::{Build, Engine};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use std::time::Instant;

const STEREO: &str = r#"
[[node]]
id = "a"
kind = "osc"
freq = 220.0
gain = 0.2
[[node]]
id = "b"
kind = "fm"
freq = 330.0
ratio = 2.0
index = 1.5
gain = 0.2
[[node]]
id = "c"
kind = "osc"
freq = 440.0
gain = 0.15
[[node]]
id = "d"
kind = "fm"
freq = 110.0
ratio = 3.0
index = 2.0
gain = 0.2
[[node]]
id = "mix"
kind = "mixer"
[[node]]
id = "out"
kind = "audio_out"
volume = 1.0
[[link]]
from = "a"
to = "mix.in1"
[[link]]
from = "b"
to = "mix.in2"
[[link]]
from = "c"
to = "mix.in3"
[[link]]
from = "d"
to = "mix.in4"
[[link]]
from = "mix"
to = "out.in"
"#;

#[test]
#[ignore]
fn the_stereo_scene_costs() {
    let g = Graph::build(&Scene::parse(STEREO).unwrap()).unwrap();
    let mut times = Vec::new();
    for _ in 0..7 {
        let (mut e, _) = Engine::build_with(&g, 48_000.0, 1.0, Build::detect()).unwrap();
        let mut buf = vec![0.0f32; 128 * 2];
        let t = Instant::now();
        for _ in 0..(10 * 48_000 / 128) {
            e.render(&mut buf, 2, |x| x);
        }
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("stereo scene, 10 s in 128-frame blocks: median {:.1} ms (min {:.1}, max {:.1})", times[3], times[0], times[6]);
}
