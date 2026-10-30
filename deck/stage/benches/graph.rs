//! `cargo bench -p deck-stage` – the stage's graph workloads (#183): the audio block and the control step, µs per call
//! (the median of 7 runs, like deck/lab). The numbers belong in the issue / the README's "Measured", not in CI.
use deck_stage::audio::{Build, Engine, Track};
use deck_stage::control::Controls;
use deck_stage::graph::Graph;
use deck_stage::scene::{Scene, DEFAULT_SCENE};
use deck_stage::tempo;
use std::sync::Arc;
use std::time::Instant;

const RATE: f32 = 48_000.0;
/// a PipeWire quantum / an AudioWorklet block
const FRAMES: usize = 128;

/// µs per call: `iters` calls per run, 7 runs, the median
fn time(iters: u32, mut f: impl FnMut()) -> f64 {
    for _ in 0..iters / 10 + 1 {
        f();
    }
    let mut runs: Vec<f64> = (0..7)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_nanos() as f64 / iters as f64 / 1000.0
        })
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    runs[3]
}

/// `n` voices of `kind` (≤ 16), each into a mixer, the mixers into one more, then the output; `cv` = one sequencer
/// pitches them all
pub fn voices(kind: &str, n: usize, cv: bool) -> String {
    let mut s = String::new();
    if cv {
        s += "[[node]]\nid=\"seq\"\nkind=\"seq\"\nbpm=120\ndivision=4\ngate=0.6\nsteps=[45, 0, 52, 57, 55, 0, 60, 64]\n";
    }
    for i in 0..n {
        s += &format!("[[node]]\nid=\"v{i}\"\nkind=\"{kind}\"\nfreq={}\ngain=0.05\nratio=2\nindex=1.5\n", 110.0 + 37.0 * i as f32);
        s += &format!("[[link]]\nfrom=\"v{i}\"\nto=\"m{}.in{}\"\n", i / 4, i % 4 + 1);
        if cv {
            s += &format!("[[link]]\nfrom=\"seq.cv\"\nto=\"v{i}.cv\"\n");
        }
    }
    let mixers = n.div_ceil(4);
    for m in 0..mixers {
        s += &format!("[[node]]\nid=\"m{m}\"\nkind=\"mixer\"\n[[link]]\nfrom=\"m{m}\"\nto=\"sum.in{}\"\n", m + 1);
    }
    s + "[[node]]\nid=\"sum\"\nkind=\"mixer\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n[[link]]\nfrom=\"sum\"\nto=\"out.in\"\n"
}

/// `levels` × `width` LFOs: each feeds the rate of the one below it, the shapes cycle sine / saw / square
pub fn lfos(levels: usize, width: usize) -> String {
    let mut s = String::new();
    for l in 0..levels {
        for w in 0..width {
            let shape = ["sine", "saw", "square"][(l + w) % 3];
            s += &format!("[[node]]\nid=\"l{l}_{w}\"\nkind=\"lfo\"\nrate={}\ndepth=0.9\nshape=\"{shape}\"\n", 0.1 + 0.01 * w as f32);
            if l > 0 {
                s += &format!("[[link]]\nfrom=\"l{}_{w}\"\nto=\"l{l}_{w}.rate\"\nscale=0.5\noffset=0.2\n", l - 1);
            }
        }
    }
    s
}

fn graph(text: &str) -> Graph {
    Graph::build(&Scene::parse(text).unwrap()).unwrap()
}

/// the audio workload on the patcher (the stage before #183), the batch runtime's baseline loops and its AVX2 loops
/// (on a CPU without AVX2 the last column runs the baseline loops)
fn audio(name: &str, text: &str) {
    let builds = [Build::patcher(), Build { batch: true, avx2: false }, Build { batch: true, avx2: true }];
    let us: Vec<String> = builds.iter().map(|&b| format!("{:.2}", audio_block(text, b))).collect();
    println!("| audio block {FRAMES}, {name} | {} |", us.join(" | "));
}

fn audio_block(text: &str, build: Build) -> f64 {
    let g = graph(text);
    let (mut e, mut h) = Engine::build_with(&g, RATE, 1.0, build).unwrap();
    if let Some(m) = h.music.first_mut() {
        let t = Track { rate: 44_100.0, l: (0..441_000).map(|i| (i as f32 * 0.01).sin() * 0.3).collect(), r: vec![0.1; 441_000] };
        m.tracks.push(Arc::new(t)).ok().unwrap();
    }
    let mut buf = vec![0.0f32; FRAMES * 2];
    time(20_000, || {
        e.render(&mut buf, 2, |x| x);
        for (_, tap) in h.taps.iter_mut() {
            while tap.pop().is_ok() {}
        }
    })
}

/// the control step on the batches (the patcher's numbers before #183: deck/stage/README.md "Measured")
fn control(name: &str, text: &str) {
    let g = graph(text);
    let mut c = Controls::new(&g, None);
    let now = tempo::Now::default();
    let us = time(20_000, || c.update(1.0 / 60.0, 0, &now));
    println!("| control step, {name} | – | {us:.2} | – |");
}

fn main() {
    println!("| workload, µs | patcher | batch | batch AVX2 |\n|---|---|---|---|");
    audio("the default scene", DEFAULT_SCENE);
    audio("16 FM voices", &voices("fm", 16, false));
    audio("16 FM voices on a sequencer", &voices("fm", 16, true));
    audio("16 sine oscillators", &voices("osc", 16, false));
    control("300 LFOs wide (3 levels × 100)", &lfos(3, 100));
    control("300 LFOs deep (300 levels)", &lfos(300, 1));
}
