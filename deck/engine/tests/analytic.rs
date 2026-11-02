//! The engine's kinds against analytic references (plan 3.2, #317): what the rewritten units and the true `fm` must do,
//! measured on the engine's own output – through the graph, the envelope, the master and the limiter – with coherent FFTs.
//! The units' own tests (harmonic levels to 0.0004 dB, aliasing, the filter's bilinear response) are in deck-core's dsp.rs;
//! here is what only the engine can get wrong: the wiring (which wave is which), the frequency modulation and the noise
//! nodes' lowpass.
use deck_engine::{Build, Engine};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

const RATE: f32 = 48_000.0;
const N: usize = 16_384;

/// The left channel of `toml` after master and limiter: `skip` frames discarded (the envelope's attack, the limiter's
/// look-ahead), then `frames` returned.
fn render(toml: &str, skip: usize, frames: usize) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(toml).unwrap()).unwrap();
    let (mut engine, _h) = Engine::build_with(&graph, RATE, 1.0, Build::detect()).unwrap();
    let mut buf = vec![0.0f32; 2 * 4096];
    let mut all = Vec::with_capacity(skip + frames);
    while all.len() < skip + frames {
        engine.render(&mut buf, 2, |x| x);
        all.extend(buf.chunks_exact(2).map(|f| f[0]));
    }
    all[skip..skip + frames].to_vec()
}

/// amplitude spectrum, bins 0..N/2 (a full-scale sine bin reads its amplitude)
fn spectrum(x: &[f32]) -> Vec<f64> {
    let mut buf: Vec<Complex<f64>> = x.iter().map(|&v| Complex::new(f64::from(v), 0.0)).collect();
    FftPlanner::new().plan_fft_forward(x.len()).process(&mut buf);
    buf[..x.len() / 2].iter().map(|c| c.norm() * 2.0 / x.len() as f64).collect()
}

fn hz(bin: usize) -> f64 {
    bin as f64 * f64::from(RATE) / N as f64
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1e-30).log10()
}

const OUT: &str = "[[node]]\nid = \"out\"\nkind = \"audio_out\"\nvolume = 1.0\n";

#[test]
fn the_waves_are_the_waves_they_are_named() {
    // bin 211 = 618 Hz, gain 0.4: harmonic k of a unit saw / square / triangle is 2/(πk), 4/(πk) (odd), 8/(π²k²) (odd)
    for (wave, harmonic) in [("saw", 2.0 / PI), ("square", 4.0 / PI), ("triangle", 8.0 / (PI * PI))] {
        let toml = format!("[[node]]\nid = \"o\"\nkind = \"osc\"\nwave = \"{wave}\"\nfreq = {}\ngain = 0.4\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n{OUT}", hz(211));
        let s = spectrum(&render(&toml, 4096, N));
        let first = s[211];
        assert!((first - 0.4 * harmonic).abs() < 0.4 * harmonic * 0.002, "{wave}: the fundamental is {first}, want {}", 0.4 * harmonic);
        let k3 = s[3 * 211] / first;
        let want3 = match wave {
            "triangle" => 1.0 / 9.0,
            _ => 1.0 / 3.0,
        };
        assert!((k3 - want3).abs() < want3 * 0.002, "{wave}: the third harmonic is {k3} of the first, want {want3}");
        if wave != "saw" {
            assert!(db(s[2 * 211] / first) < -70.0, "{wave}: no even harmonics");
        }
    }
}

/// J_n(x) by the integral (1/π) ∫₀^π cos(nτ − x sin τ) dτ – the trapezoid rule converges geometrically for this integrand
fn bessel(n: i32, x: f64) -> f64 {
    let m = 4096;
    (0..=m)
        .map(|i| {
            let t = PI * i as f64 / m as f64;
            let w = if i == 0 || i == m { 0.5 } else { 1.0 };
            w * (f64::from(n) * t - x * t.sin()).cos()
        })
        .sum::<f64>()
        / m as f64
}

/// True frequency modulation (owner 2026-11-02, D18): the carrier's instantaneous frequency is f + index · fm · sin(modulator)
/// with fm = f · ratio, so the modulation index in Bessel's sense is `index` (deviation / fm = index · fm / fm) and the lines
/// stand at f + n · fm with the amplitudes J_n(index). Measured on the engine's output at an index below 1 (the old fold's
/// range too) and at 6 (where the instantaneous frequency goes negative and the old `abs()` fold bent the spectrum).
#[test]
fn fm_sidebands_follow_bessel() {
    // f = bin 2000 (5.86 kHz), ratio 1/8 → fm = bin 250 (1.46 kHz): n · fm for |n| ≤ 5 stays inside the spectrum, and the
    // negative frequencies' reflections (from n ≤ −9) land far from those lines
    for index in [0.8f32, 2.0, 6.0] {
        let toml = format!(
            "[[node]]\nid = \"f\"\nkind = \"fm\"\nfreq = {}\nratio = 0.125\nindex = {index}\ngain = 0.3\n[[link]]\nfrom = \"f\"\nto = \"out.in\"\n{OUT}",
            hz(2000)
        );
        let s = spectrum(&render(&toml, 8192, N));
        for n in -5..=5i32 {
            let bin = (2000 + 250 * n) as usize;
            let want = 0.3 * bessel(n, f64::from(index)).abs();
            assert!((s[bin] - want).abs() < 0.002, "index {index}: the line n = {n} (bin {bin}) is {:.4}, Bessel says {want:.4}", s[bin]);
        }
    }
}

#[test]
fn the_noise_nodes_lowpass_has_the_filters_response() {
    // white noise through the node's lowpass (cutoff 1 kHz, Q 0.7): the mean power around 2 kHz and 4 kHz is down by the
    // analogue response's bilinear image – the ratio between them within 1 dB
    let toml = format!("[[node]]\nid = \"n\"\nkind = \"noise\"\ncutoff = 1000.0\nq = 0.7\ngain = 0.5\n[[link]]\nfrom = \"n\"\nto = \"out.in\"\n{OUT}");
    let x = render(&toml, 4096, 1 << 19);
    let power = |lo: f64, hi: f64| {
        // an averaged periodogram: Welch, Hann, 4096 points
        let l = 4096;
        let window: Vec<f64> = (0..l).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / l as f64).cos()).collect();
        let fft = FftPlanner::new().plan_fft_forward(l);
        let (a, b) = ((lo / f64::from(RATE) * l as f64) as usize, (hi / f64::from(RATE) * l as f64) as usize);
        let (mut sum, mut count) = (0.0, 0);
        let mut at = 0;
        while at + l <= x.len() {
            let mut buf: Vec<Complex<f64>> = (0..l).map(|i| Complex::new(f64::from(x[at + i]) * window[i], 0.0)).collect();
            fft.process(&mut buf);
            sum += buf[a..b].iter().map(|c| c.norm_sqr()).sum::<f64>() / (b - a) as f64;
            count += 1;
            at += l / 2;
        }
        sum / count as f64
    };
    let response = |f: f64| {
        let g = (PI * 1000.0 / f64::from(RATE)).tan();
        let w = (PI * f / f64::from(RATE)).tan() / g;
        1.0 / ((1.0 - w * w).powi(2) + (w / 0.7).powi(2))
    };
    let measured = 10.0 * (power(1900.0, 2100.0) / power(3800.0, 4200.0)).log10();
    let ideal = 10.0 * (response(2000.0) / response(4000.0)).log10();
    assert!((measured - ideal).abs() < 1.0, "2 kHz over 4 kHz: {measured:.2} dB, the filter's response says {ideal:.2} dB");
}

#[test]
fn noise_is_seeded_and_every_node_has_its_own() {
    let one = |extra: &str| {
        format!("[[node]]\nid = \"n\"\nkind = \"noise\"\ncolor = \"pink\"\ncutoff = 8000.0\ngain = 0.3\n{extra}[[link]]\nfrom = \"n\"\nto = \"out.in\"\n{OUT}")
    };
    let (a, b) = (render(&one(""), 0, 4096), render(&one(""), 0, 4096));
    assert!(a == b, "a scene plays the same noise every time");
    assert!(a != render(&one("seed = 2\n"), 0, 4096), "another seed, other noise");
    // two noise nodes in one graph are not the same noise (their place in the graph is in the seed)
    let two = format!(
        "[[node]]\nid = \"a\"\nkind = \"noise\"\ngain = 0.3\n[[node]]\nid = \"b\"\nkind = \"noise\"\ngain = 0.3\n[[node]]\nid = \"m\"\nkind = \"mixer\"\ngain1 = 1.0\ngain2 = -1.0\n\
         [[link]]\nfrom = \"a\"\nto = \"m.in1\"\n[[link]]\nfrom = \"b\"\nto = \"m.in2\"\n[[link]]\nfrom = \"m\"\nto = \"out.in\"\n{OUT}"
    );
    let diff = render(&two, 4096, 4096);
    assert!(diff.iter().any(|v| v.abs() > 0.01), "a − b is not silence: the two nodes play different noise");
}
