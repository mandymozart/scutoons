//! The zipper check (#188, owner 2026-10-31: "the audio engine sometimes crackles"): the cathedral's audio rendered
//! offline into a WAV – 10 s at 48 kHz in 128-frame blocks (the deck's PipeWire quantum), the frame thread's control
//! step every 800 frames (60 fps) with a hand on the knobs: voice.index, voice.gain, drone.freq and mix.gain2 swept
//! like a drag, a few hard jumps, and the incense LFO sped up (sway.rate 2 Hz, its cutoff link a staircase per frame).
//! Prints the largest sample-to-sample step and second difference of the output (left channel). `--tonal` mutes the
//! incense (pink noise steps sample to sample by itself), so the numbers show the zipper alone.
//!
//!   cargo run --release -p deck-stage --example render-wav -- out.wav [--tonal]
use deck_stage::audio::Engine;
use deck_stage::control::Controls;
use deck_stage::graph::Graph;
use deck_stage::scene::{Scene, CATHEDRAL_SCENE};
use deck_stage::tempo;
use std::io::Write;

const RATE: f32 = 48_000.0;
const SECONDS: usize = 10;
const BLOCK: usize = 128;
const FRAME: usize = 800;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args.iter().find(|a| !a.starts_with("--")).cloned().unwrap_or_else(|| "render.wav".into());
    let tonal = args.iter().any(|a| a == "--tonal");
    let scene = deck_stage::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE)?);
    let graph = Graph::build(&scene)?;
    let (mut engine, mut handles) = Engine::build(&graph, RATE, 1.0)?;
    let mut ctl = Controls::new(&graph, Some(&mut handles));
    let at = |path: &str| {
        let (id, port) = path.split_once('.').expect("node.port");
        let n = graph.nodes.iter().position(|x| x.id == id).expect("a node");
        (n, graph.nodes[n].kind.input(port).expect("a port"))
    };
    let (index, gain, drone, mix2, sway, incense) = (at("voice.index"), at("voice.gain"), at("drone.freq"), at("mix.gain2"), at("sway.rate"), at("incense.gain"));
    ctl.set(sway.0, sway.1, 2.0);
    if tonal {
        ctl.set(incense.0, incense.1, 0.0);
    }
    let total = SECONDS * RATE as usize;
    let mut left = Vec::with_capacity(total);
    let mut buf = vec![0.0f32; BLOCK * 2];
    let mut done = 0;
    while done < total {
        if done % FRAME == 0 {
            let t = done as f32 / RATE;
            // a hand: slow sweeps (a drag's 60 steps a second) and, every 2.5 s, a hard jump
            let tri = |p: f32| 1.0 - (2.0 * (t / p).fract() - 1.0).abs();
            let jump = (t / 2.5) as usize % 2 == 1;
            ctl.set(index.0, index.1, 8.0 * tri(3.0));
            ctl.set(gain.0, gain.1, if jump { 0.3 } else { 0.05 + 0.2 * tri(1.7) });
            ctl.set(drone.0, drone.1, 30.0 * (220.0f32 / 30.0).powf(tri(4.0)));
            ctl.set(mix2.0, mix2.1, if (t * 3.0) as usize % 2 == 0 { 1.0 } else { 0.4 });
            let beat = tempo::Now { bpm: 84.0, pos: tempo::position(t as f64 * 1.4, 4), per_bar: 4, running: true };
            ctl.update(FRAME as f32 / RATE, 0, &beat);
        }
        engine.render(&mut buf, 2, |x| x);
        left.extend(buf.chunks_exact(2).map(|f| f[0]));
        done += BLOCK;
    }
    let step = left.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    let curve = left.windows(3).map(|w| (w[2] - 2.0 * w[1] + w[0]).abs()).fold(0.0f32, f32::max);
    let peak = left.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    println!("{out}: {SECONDS} s{}, peak {peak:.4}, max |x[n] - x[n-1]| {step:.5}, max |second difference| {curve:.5}", if tonal { " (tonal)" } else { "" });
    write_wav(&out, &left)?;
    Ok(())
}

/// a mono 32-bit float WAV
fn write_wav(path: &str, x: &[f32]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data = (x.len() * 4) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&3u16.to_le_bytes())?; // IEEE float
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&(RATE as u32).to_le_bytes())?;
    f.write_all(&(RATE as u32 * 4).to_le_bytes())?;
    f.write_all(&4u16.to_le_bytes())?;
    f.write_all(&32u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data.to_le_bytes())?;
    for v in x {
        f.write_all(&v.to_le_bytes())?;
    }
    f.flush()
}
