//! The `voice` node measured on the engine's output (through the graph, the envelope, the master and the limiter): each
//! vowel's spectral peaks sit at its F1 and F2, the pitch follows `freq` and the `cv`, a glide reaches its note in its
//! time, a closed gate is silence.
use deck_engine::{Build, Engine};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use rustfft::{num_complex::Complex, FftPlanner};

const RATE: f32 = 48_000.0;
const N: usize = 16_384;
const OUT: &str = "[[node]]\nid = \"out\"\nkind = \"audio_out\"\nvolume = 1.0\n[[link]]\nfrom = \"v\"\nto = \"out.in\"\n";

/// the left channel of `toml`, frames `skip..skip + frames`
fn render(toml: &str, skip: usize, frames: usize) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(toml).unwrap()).unwrap();
    let (mut engine, _h) = Engine::build_with(&graph, RATE, 1.0, Build::detect()).unwrap();
    let mut buf = vec![0.0f32; 2 * 4096];
    let mut all = Vec::with_capacity(skip + frames + 4096);
    while all.len() < skip + frames {
        engine.render(&mut buf, 2, |x| x);
        all.extend(buf.chunks_exact(2).map(|f| f[0]));
    }
    all[skip..skip + frames].to_vec()
}

fn spectrum(x: &[f32]) -> Vec<f64> {
    let mut buf: Vec<Complex<f64>> = x.iter().map(|&v| Complex::new(f64::from(v), 0.0)).collect();
    FftPlanner::new().plan_fft_forward(x.len()).process(&mut buf);
    buf[..x.len() / 2].iter().map(|c| c.norm() * 2.0 / x.len() as f64).collect()
}

fn hz(bin: usize) -> f32 {
    bin as f32 * RATE / N as f32
}

/// a held note without vibrato or breath
fn note(freq: f32, vowel: f32) -> String {
    format!("[[node]]\nid = \"v\"\nkind = \"voice\"\nfreq = {freq}\nvowel = {vowel}\nvibrato = 0.0\nbreath = 0.0\ngain = 0.5\n{OUT}")
}

/// the pitch of `x` by autocorrelation (the lag of the strongest peak between 60 and 1600 Hz, parabolically refined)
fn pitch(x: &[f32]) -> f32 {
    let r = |lag: usize| x[..x.len() - lag].iter().zip(&x[lag..]).map(|(a, b)| f64::from(a * b)).sum::<f64>();
    let (lo, hi) = ((RATE / 1600.0) as usize, (RATE / 60.0) as usize);
    let rs: Vec<f64> = (lo - 1..=hi + 1).map(r).collect();
    let best = (1..rs.len() - 1).max_by(|&a, &b| rs[a].total_cmp(&rs[b])).unwrap();
    let (a, b, c) = (rs[best - 1], rs[best], rs[best + 1]);
    let lag = (best + lo - 1) as f64 + 0.5 * (a - c) / (a - 2.0 * b + c);
    RATE / lag as f32
}

#[test]
fn every_vowel_peaks_at_its_first_two_formants() {
    let f0 = 38; // ≈ 111 Hz: dense harmonics, a coherent window
    for (index, v) in deck_engine::voice::VOWELS.iter().enumerate() {
        let s = spectrum(&render(&note(hz(f0), index as f32), 8192, N));
        let harmonics: Vec<(f32, f64)> = (1..(5000 / 111)).map(|k| (hz(k * f0), s[k * f0])).collect();
        let strongest = |lo: f32, hi: f32| harmonics.iter().filter(|(f, _)| (lo..hi).contains(f)).max_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0;
        let [f1, f2, f3, _] = v.freq;
        let tolerance = |f: f32| hz(f0) / 2.0 + 0.1 * f;
        let p1 = strongest(f1 * 0.5, (f1 + f2) / 2.0);
        let p2 = strongest((f1 + f2) / 2.0, (f2 + f3) / 2.0);
        assert!((p1 - f1).abs() <= tolerance(f1), "{}: F1 peak at {p1} Hz, want {f1}", v.name);
        assert!((p2 - f2).abs() <= tolerance(f2), "{}: F2 peak at {p2} Hz, want {f2}", v.name);
    }
}

#[test]
fn the_pitch_follows_freq_and_the_cv() {
    for freq in [110.0, 262.0, 523.25, 880.0] {
        let p = pitch(&render(&note(freq, 2.0), 4800, 4800));
        assert!((p / freq - 1.0).abs() < 0.003, "freq {freq}: heard {p}");
    }
    // the harmonics of a coherent note carry all its energy (no stray partials)
    let s = spectrum(&render(&note(hz(100), 1.0), 8192, N));
    let total: f64 = s.iter().skip(1).map(|a| a * a).sum();
    let harmonic: f64 = s.iter().enumerate().skip(1).filter(|(i, _)| i % 100 == 0).map(|(_, a)| a * a).sum();
    assert!(harmonic / total > 0.999, "harmonic share {}", harmonic / total);
    // a seq's A4 into the cv
    let cv = format!(
        "[[node]]\nid = \"s\"\nkind = \"seq\"\nbpm = 60.0\ndivision = 1\ngate = 1.0\nsteps = [69]\n\
         [[node]]\nid = \"v\"\nkind = \"voice\"\nvibrato = 0.0\nbreath = 0.0\ngain = 0.5\n[[link]]\nfrom = \"s.cv\"\nto = \"v.cv\"\n{OUT}"
    );
    let p = pitch(&render(&cv, 4800, 4800));
    assert!((p / 440.0 - 1.0).abs() < 0.003, "cv A4: heard {p}");
}

#[test]
fn a_glide_reaches_its_note_in_its_time() {
    // A3 for a second, then A4 with a 0.2 s glide (a step a second)
    let toml = format!(
        "[[node]]\nid = \"s\"\nkind = \"seq\"\nbpm = 60.0\ndivision = 1\ngate = 1.0\nsteps = [57, 69]\n\
         [[node]]\nid = \"v\"\nkind = \"voice\"\nglide = 0.2\nvibrato = 0.0\nbreath = 0.0\ngain = 0.5\n[[link]]\nfrom = \"s.cv\"\nto = \"v.cv\"\n{OUT}"
    );
    let x = render(&toml, 0, 2 * 48_000);
    let at = |t: f32, len: usize| {
        let c = (t * RATE) as usize;
        pitch(&x[c - len / 2..c + len / 2])
    };
    assert!((at(0.9, 4800) / 220.0 - 1.0).abs() < 0.003, "before: {}", at(0.9, 4800));
    let mid = 220.0 * 2f32.sqrt();
    assert!((at(1.1, 960) / mid - 1.0).abs() < 0.03, "half way: {} want {mid}", at(1.1, 960));
    assert!(at(1.17, 960) < 435.0, "not there before its time: {}", at(1.17, 960));
    assert!((at(1.25, 2400) / 440.0 - 1.0).abs() < 0.003, "arrived: {}", at(1.25, 2400));
}

#[test]
fn a_closed_gate_is_silence() {
    let toml = format!(
        "[[node]]\nid = \"s\"\nkind = \"seq\"\nbpm = 60.0\nsteps = [0]\n\
         [[node]]\nid = \"v\"\nkind = \"voice\"\nbreath = 1.0\ngain = 1.0\n[[link]]\nfrom = \"s.cv\"\nto = \"v.cv\"\n{OUT}"
    );
    assert!(render(&toml, 0, 48_000).iter().all(|&x| x == 0.0));
}

#[cfg(feature = "events")]
/// #819: a walk whose `open` is 0 (the default character rig's trigger, shut) leaves the voice silent; open, it sings
#[test]
fn a_walk_shut_by_its_open_is_silence() {
    let walk = |open: f32| {
        format!(
            "[[node]]\nid = \"w\"\nkind = \"walk\"\nseed = 491\nbase = 262.0\nspread = 0.4\nopen = {open}\n\
             [[node]]\nid = \"v\"\nkind = \"voice\"\nbreath = 1.0\ngain = 1.0\n[[link]]\nfrom = \"w.cv\"\nto = \"v.cv\"\n{OUT}"
        )
    };
    assert!(render(&walk(0.0), 0, 48_000).iter().all(|&x| x == 0.0), "shut");
    assert!(render(&walk(1.0), 0, 48_000).iter().any(|&x| x.abs() > 1e-3), "open: it sings");
}

#[test]
fn a_sustained_vowel_is_loud_but_not_clipped() {
    for v in 0..6 {
        let x = render(&note(262.0, v as f32).replace("gain = 0.5", "gain = 1.0"), 4800, 48_000);
        let peak = x.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((0.2..0.95).contains(&peak), "vowel {v}: peak {peak}");
    }
}
