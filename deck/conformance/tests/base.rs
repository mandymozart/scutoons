//! (d) The base check of plan §3.1 / §3.8 P0d, native half: the lab engine's planar N-channel pool against the native
//! deck's `Engine` on today's stereo scene (hash-equal, time within 5 %), the routing graph's exact sums, and the two
//! saws through the analytic FFT test. (The wasm half – native ≡ wasm32 + simd128 – is in wasm.rs.)
mod common;
use common::*;
use deck_engine_lab::{route, run_route, run_stereo, saw_polyblep, Exp, RouteScene, SawTables, STEREO_TOML};
use deck_native::audio::Build;
use rustfft::{num_complex::Complex, FftPlanner};
use std::time::Instant;

fn start_phase() -> f32 {
    deck_core::dsp::START_PHASE
}

/// the stage's stereo output, both channels interleaved (the lab renders L R too)
fn stage_stereo(block: usize, frames: usize, build: Build) -> Vec<f32> {
    let graph = stage_graph(STEREO_TOML);
    let (mut engine, _h) = deck_native::audio::Engine::build_with(&graph, RATE, 1.0, build).unwrap();
    let mut out = Vec::with_capacity(frames * 2);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        engine.render(&mut buf[..n * 2], 2, |x| x);
        out.extend_from_slice(&buf[..n * 2]);
        done += n;
    }
    out
}

#[test]
fn the_lab_pool_renders_todays_stereo_scene_bit_for_bit() {
    for &block in &PATTERNS {
        let stage = stage_stereo(block, FRAMES, Build::detect());
        let lab = run_stereo(FRAMES, block, start_phase(), Exp::Std, 0);
        let wide = run_stereo(FRAMES, block, start_phase(), Exp::Std, 12);
        let d = diff(&stage, &lab);
        println!("stereo scene, block {block}: stage {:016x} lab {:016x} wide pool {:016x} (max abs {:.3e})", hash(&stage), hash(&lab), hash(&wide), d.max_abs);
        assert!(peak(&stage) > 0.1);
        assert!(d.exact, "block {block}: the lab's pool reproduces the stage's samples (first difference at {:?})", d.first);
        assert_eq!(hash(&lab), hash(&wide), "a wider pool changes nothing");
    }
}

fn best_of<F: FnMut()>(n: usize, mut f: F) -> f64 {
    (0..n)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64()
        })
        .fold(f64::INFINITY, f64::min)
}

/// Time of 10 s of the stereo scene in 128-frame blocks: the stage's Engine, the lab on the 2-channel-per-buffer pool, the
/// same pool widened by 12 unused channels. Best of 9 (the machine's noise is one-sided).
#[test]
fn the_pool_costs_the_stereo_scene_nothing() {
    let frames = 10 * 48_000;
    let graph = stage_graph(STEREO_TOML);
    let sp = start_phase();
    let stage = best_of(9, || {
        let (mut engine, _h) = deck_native::audio::Engine::build_with(&graph, RATE, 1.0, Build::detect()).unwrap();
        let mut buf = vec![0.0f32; 128 * 2];
        for _ in 0..frames / 128 {
            engine.render(&mut buf, 2, |x| x);
        }
        std::hint::black_box(&buf);
    });
    let lab = best_of(9, || {
        std::hint::black_box(run_stereo(frames, 128, sp, Exp::Std, 0));
    });
    let wide = best_of(9, || {
        std::hint::black_box(run_stereo(frames, 128, sp, Exp::Std, 12));
    });
    let ms = |s: f64| s * 1000.0;
    let md = format!(
        "| engine | 10 s of the stereo scene (best of 9) | vs stage |\n|---|---|---|\n| stage `Engine` ({}) | {:.2} ms | – |\n| lab, planar pool | {:.2} ms | {:+.1} % |\n| lab, pool + 12 unused channels | {:.2} ms | {:+.1} % |\n",
        Build::detect().label(),
        ms(stage),
        ms(lab),
        (lab / stage - 1.0) * 100.0,
        ms(wide),
        (wide / stage - 1.0) * 100.0
    );
    std::fs::write(report_path("pool-time.md"), &md).unwrap();
    println!("{md}");
    // not a gate (a loaded machine moves it): a gross regression only
    assert!(lab < stage * 2.0, "the lab pool is not twice as slow as the stage");
}

#[test]
fn the_routing_graph_sums_exactly() {
    let mut scene = RouteScene::new(start_phase(), Exp::Std);
    let mut out = Vec::new();
    scene.block(1024, &mut out);
    scene.block(1024, &mut out); // the second block: the envelopes are up
    let (p, d) = (&scene.pool, &scene.dev);
    for i in 0..1024 {
        // the send: channels 1/2 of A land on channels 3/4 of the send bus, scaled; its channels 1/2 stay silent
        assert_eq!(p.ch(route::SEND[0])[i], 0.0);
        assert_eq!(p.ch(route::SEND[2])[i], 0.0 + p.ch(route::A[0])[i] * route::SEND_GAIN);
        assert_eq!(p.ch(route::SEND[3])[i], 0.0 + p.ch(route::A[1])[i] * route::SEND_GAIN);
        // the `out` on device channels 3–4 carries exactly the send bus's channels 3/4
        assert_eq!(d.ch(2)[i], p.ch(route::SEND[2])[i]);
        assert_eq!(d.ch(3)[i], p.ch(route::SEND[3])[i]);
        // the folder: A + B, plus the 5.1 fold on the same device pair
        let folder = 0.0 + p.ch(route::A[0])[i] * 1.0 + p.ch(route::B[0])[i] * 1.0;
        assert_eq!(p.ch(route::FOLDER[0])[i], folder);
        assert_eq!(d.ch(0)[i], folder + p.ch(route::FOLD51[0])[i]);
        // 5.1 → stereo: Lo = L + C/√2 + Ls/√2, the LFE (channel 4 of the bus) is dropped
        let b = |k: usize| p.ch(route::BUS51[k])[i];
        let lo = ((0.0 + b(0) * 1.0) + b(2) * std::f32::consts::FRAC_1_SQRT_2) + b(4) * std::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(p.ch(route::FOLD51[0])[i], lo);
    }
    // VBAP: power-normalised, only the adjacent pair, 100° lies between the speakers at 45° and 135°
    let g = deck_engine_lab::vbap4(route::AZIMUTH);
    let power: f32 = g.iter().map(|x| x * x).sum();
    assert!((power - 1.0).abs() < 1e-6, "power {power}");
    assert!(g[0] > 0.0 && g[1] > 0.0 && g[2] == 0.0 && g[3] == 0.0, "{g:?}");
    assert!(g[1] > g[0], "100° is nearer 135° than 45°: {g:?}");
    for (k, gk) in g.iter().enumerate() {
        assert_eq!(d.ch(4 + k)[10], 0.0 + p.ch(route::MONO)[10] * gk);
    }
    // the same block cut in other ways gives the same samples
    let a = run_route(FRAMES, 128, start_phase(), Exp::Std);
    let b = run_route(FRAMES, 37, start_phase(), Exp::Std);
    let c = run_route(FRAMES, 1024, start_phase(), Exp::Std);
    println!("routing graph hashes: 128 {:016x} · 37 {:016x} · 1024 {:016x}", hash(&a), hash(&b), hash(&c));
    assert_eq!(a.len(), FRAMES * route::DEVICE);
    assert_eq!(hash(&a), hash(&c), "128 and 1024 agree (envelopes and gains are per-sample)");
    assert_eq!(hash(&a), hash(&b), "128 and 37 agree");
}

// ── the saws through the analytic FFT ────────────────────────────────────────────────────────────────────────────────

struct SawStats {
    /// the worst error of a harmonic's level against the ideal 2/(πk), in dB, over the harmonics up to 12 kHz
    harmonic_err_db: f32,
    /// the highest harmonic (Hz) such that it and all lower ones are within 3 dB of the ideal
    bandwidth_hz: f32,
    /// energy outside the harmonic bins against the harmonics' energy, in dB (the aliasing)
    alias_db: f32,
}

/// A coherent window: `bin` periods fit the 16384 samples, so a perfect saw has lines at multiples of `bin` only.
fn saw_stats(x: &[f32], bin: usize) -> SawStats {
    const N: usize = 16_384;
    let mut buf: Vec<Complex<f32>> = x[..N].iter().map(|v| Complex::new(*v, 0.0)).collect();
    FftPlanner::<f32>::new().plan_fft_forward(N).process(&mut buf);
    let amp = |k: usize| 2.0 * buf[k].norm() / N as f32; // a sine of amplitude A shows A
    let (mut harm, mut other, mut worst, mut bandwidth, mut intact) = (0.0f64, 0.0f64, 0.0f32, 0.0f32, true);
    let max_k = (N / 2 - 1) / bin;
    let hz = |k: usize| k as f32 * RATE / N as f32;
    for k in 1..N / 2 {
        let e = f64::from(amp(k)).powi(2);
        if k % bin == 0 && k / bin <= max_k {
            harm += e;
            let ideal = 2.0 / (std::f32::consts::PI * (k / bin) as f32);
            let err = (20.0 * (amp(k).max(1e-12) / ideal).log10()).abs();
            if hz(k) <= 12_000.0 {
                worst = worst.max(err);
            }
            intact &= err <= 3.0;
            if intact {
                bandwidth = hz(k);
            }
        } else {
            other += e;
        }
    }
    SawStats { harmonic_err_db: worst, bandwidth_hz: bandwidth, alias_db: (10.0 * (other / harm).log10()) as f32 }
}

#[test]
fn polyblep_against_wavetable_saws() {
    let tables_ms = best_of(5, || {
        std::hint::black_box(SawTables::new());
    }) * 1000.0;
    let tables = SawTables::new();
    let mut md = format!("Wavetable build at load: {tables_ms:.1} ms (11 octave tables of 2048, deck-core's sine).\n\n| saw | pitch | worst harmonic level error to 12 kHz (dB) | harmonics intact (±3 dB) up to (Hz) | energy outside the harmonics (dB re harmonics) | ns / sample |\n|---|---|---|---|---|---|\n");
    for (bin, label) in [(211usize, "618 Hz"), (1500, "4.4 kHz"), (4000, "11.7 kHz")] {
        let freq = bin as f32 * RATE / 16_384.0;
        let n = 16_384;
        let blep = saw_polyblep(freq, n);
        let table = tables.render(freq, n);
        let t_blep = best_of(5, || {
            std::hint::black_box(saw_polyblep(freq, 2_000_000));
        }) / 2_000_000.0 * 1e9;
        let t_table = best_of(5, || {
            std::hint::black_box(tables.render(freq, 2_000_000));
        }) / 2_000_000.0 * 1e9;
        for (name, x, t) in [("PolyBLEP", &blep, t_blep), ("wavetable", &table, t_table)] {
            let s = saw_stats(x, bin);
            md.push_str(&format!("| {name} | {label} | {:.2} | {:.0} | {:.1} | {:.2} |\n", s.harmonic_err_db, s.bandwidth_hz, s.alias_db, t));
            assert!(s.alias_db < -20.0, "{name} {label}: aliasing {}", s.alias_db);
        }
    }
    std::fs::write(report_path("saws.md"), &md).unwrap();
    println!("{md}");
}
