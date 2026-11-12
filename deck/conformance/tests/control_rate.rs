//! P5 (#319): the control half at block rate against the frame rate it ran at before – the per-scene golden diff. Every
//! scene of the suite with a control half, and every scene of the native deck's list (the built-ins and deck/native/scenes),
//! rendered 4 s at 48 kHz twice: the control half stepped once per 128-frame quantum (now) and at 60 Hz (the native deck's
//! video frame thread before #319, `Engine::control_at`). The table goes to `deck/target/conformance/control-rate.md`
//! (and into research/ENGINE-REFACTOR-PLAN.md P5).
//!
//! What it asserts is the point of the change: at block rate a control value that moves into an audio input leaves no
//! staircase in the sound (no zipper): the sidebands the 60 Hz staircase put at the frame rate's multiples (on a gain and
//! on a frequency) are gone, and none appears at the block rate's 375 Hz. The audio blocks glide every control input
//! across their block sample by sample (smooth.rs), so even the 60 Hz staircase reached the sound only far down.
mod common;
use common::*;
use deck_native::audio::Build;
use deck_native::graph::{Graph, Kind, PortType};
use deck_native::scene::Scene;
use std::fmt::Write;

const SECONDS: usize = 4;

/// the native deck's scene list in dev mode: the system scenes (F5's first lines), then the sandboxes (#427)
fn deck_scenes() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> =
        deck_native::switcher::BUILTIN.iter().map(|n| (n.to_string(), deck_native::switcher::builtin(n).unwrap().to_string())).collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("native").join("scenes").join("sandbox");
    for e in deck_native::switcher::entries(Some(&dir)).into_iter().skip(out.len()) {
        out.push((e.label, std::fs::read_to_string(&e.value).unwrap()));
    }
    out
}

/// what crosses from the control half into the sound: "source.port → node.port" per control link into an audio node
fn crossings(toml: &str) -> Vec<String> {
    let scene = deck_native::modules::carried_part(&Scene::parse(toml).unwrap());
    let g = Graph::build(&scene).unwrap();
    let audio = |i: usize| g.nodes[i].kind.domain() == deck_native::graph::Domain::Audio && g.nodes[i].kind != Kind::Fft;
    g.links
        .iter()
        .filter(|l| l.ty == PortType::Control && audio(l.to.0))
        .map(|l| {
            let (f, t) = (&g.nodes[l.from.0], &g.nodes[l.to.0]);
            format!("{} {}.{} → {}.{}", f.kind.name(), f.id, f.kind.outputs()[l.from.1].name, t.id, t.kind.inputs()[l.to.1].name)
        })
        .collect()
}

/// the level of `f` Hz in `x` (a Hann-windowed DFT bin by Goertzel), dB relative to a full-scale sine
fn level_db(x: &[f32], f: f64) -> f64 {
    let n = x.len();
    let w = std::f64::consts::TAU * f / RATE as f64;
    let (c, mut s1, mut s2) = (2.0 * w.cos(), 0.0f64, 0.0f64);
    for (i, &v) in x.iter().enumerate() {
        let hann = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
        let s0 = f64::from(v) * hann + c * s1 - s2;
        (s2, s1) = (s1, s0);
    }
    let mag = (s1 * s1 + s2 * s2 - c * s1 * s2).sqrt();
    20.0 * (mag * 4.0 / n as f64).max(1e-30).log10()
}

#[test]
fn block_rate_against_frame_rate_per_scene() {
    let frames = SECONDS * RATE as usize;
    let mut md = String::from(
        "| scene | control → sound | max abs | RMS of diff | diff / level | level RMS (block) | level frame / block |\n|---|---|---|---|---|---|---|\n",
    );
    let mut scenes: Vec<(String, String)> = CASES.iter().chain(CONTROL_CASES).filter(|c| c.toml.contains("lfo") || c.toml.contains("tempo")).map(|c| (c.name.to_string(), c.toml.to_string())).collect();
    scenes.extend(deck_scenes().into_iter().map(|(n, t)| (format!("deck: {n}"), t)));
    for (name, toml) in &scenes {
        let block = stage_render_at(toml, 128, frames, Build::detect(), None);
        let frame = stage_render_at(toml, 128, frames, Build::detect(), Some(60.0));
        let d = diff(&block, &frame);
        let level = rms(&block);
        let rel = if d.rms > 0.0 && level > 0.0 { format!("{:.1} dB", 20.0 * (d.rms / level).log10()) } else { "–".into() };
        let cross = crossings(toml);
        let cross = if cross.is_empty() { "none".to_string() } else { cross.join("<br>") };
        let loud = if level > 0.0 { format!("{:+.2} dB", 20.0 * (rms(&frame) / level).log10()) } else { "–".into() };
        writeln!(md, "| {name} | {cross} | {:.3e} | {:.3e} | {rel} | {level:.4} | {loud} |", d.max_abs, d.rms).unwrap();
        if d.exact && level > 0.0 {
            assert!(!crossings(toml).iter().any(|c| c.starts_with("lfo")), "{name}: an LFO into the sound must change with the rate");
        }
    }
    // the zipper: a control value that moves into a gain (the tremolo: 1 kHz, its gain swayed by a 5 Hz sine) and into a
    // frequency (a vibrato: 1 kHz ± 10 Hz at 5 Hz). Held flat between control steps, the value would be a staircase: at the
    // frame rate sidebands at 1000 ± k · 60 Hz, at the block rate at 1000 ± k · 375 Hz. Smooth, a tremolo has only
    // 1000 ± 5 Hz and a vibrato 1000 ± k · 5 Hz with Bessel weights (J_k(2): under -150 dB from k = 12, i.e. ± 60 Hz, on)
    let vibrato = concat!(
        "[[node]]
id = \"l\"
kind = \"lfo\"
rate = 5.0
depth = 20.0
[[node]]
id = \"o\"
kind = \"osc\"
gain = 0.2
",
        "[[link]]
from = \"l\"
to = \"o.freq\"
offset = 990.0
[[link]]
from = \"o\"
to = \"out.in\"
",
        "[[node]]
id = \"out\"
kind = \"audio_out\"
volume = 1.0
"
    );
    writeln!(md, "
The zipper – sidebands around the 1 kHz carrier, dB under it (the 25 ms glide of smooth.rs on both; ± 5 Hz is the modulation itself):
").unwrap();
    writeln!(md, "| sideband | tremolo, block rate | tremolo, 60 Hz | vibrato, block rate | vibrato, 60 Hz |
|---|---|---|---|---|").unwrap();
    let renders: Vec<[Vec<f32>; 2]> = [CONTROL_CASES[0].toml, vibrato]
        .iter()
        .map(|t| [stage_render_at(t, 128, frames, Build::detect(), None), stage_render_at(t, 128, frames, Build::detect(), Some(60.0))])
        .collect();
    let mut worst = [[f64::MIN; 2]; 2];
    for off in [5.0, 60.0, 120.0, 180.0, 375.0, 750.0] {
        let mut row = format!("| ± {off} Hz |");
        for (s, pair) in renders.iter().enumerate() {
            for (k, x) in pair.iter().enumerate() {
                let carrier = level_db(x, 1000.0);
                let lv = ((level_db(x, 1000.0 - off) + level_db(x, 1000.0 + off)) / 2.0) - carrier;
                write!(row, " {lv:.1} |").unwrap();
                if off > 5.0 {
                    worst[s][k] = worst[s][k].max(lv);
                }
            }
        }
        writeln!(md, "{row}").unwrap();
    }
    std::fs::write(report_path("control-rate.md"), &md).unwrap();
    println!("{md}");
    for (s, name) in ["tremolo", "vibrato"].iter().enumerate() {
        let [b, f] = worst[s];
        assert!(b < -125.0 && b < f - 20.0, "{name}: no zipper sideband at block rate ({b:.1} dB; the 60 Hz frame rate's {f:.1} dB)");
    }
}
